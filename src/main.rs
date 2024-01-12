// Simulates orbit of a small body around the earth
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use std::ops;

type Precision = f64;

const G: Precision = 6.6743e-11; // m3 kg-1 s-2
const MASS_EARTH: Precision = 5.972e24;
const EARTH_RADIUS: Precision = 6.371e6;
const DT: Precision = 10.0; // 1 second
const N_HISTORY: usize = 21;
const N_LOOKAHEAD: usize = 2000;
const THRUST: Precision = 2.0;

// Bodies have a mass, an id, a current state and a rolling history of states
#[derive(Component)]
struct Body {
    current_state: State,
    history: StateHistory,
    mass: Precision,
    id: usize,
}

#[derive(Copy, Clone)]
struct State {
    vx: Precision,
    vy: Precision,
    x: Precision,
    y: Precision,
}

impl State {
    fn new(x0: Precision, y0: Precision, vx0: Precision, vy0: Precision) -> Self {
        Self {
            x: x0,
            y: y0,
            vx: vx0,
            vy: vy0,
        }
    }
}

struct StateHistory([Option<State>; N_HISTORY]);

impl StateHistory {
    fn new() -> Self {
        StateHistory([None; N_HISTORY])
    }

    fn push(&mut self, state: State) {
        self.0.rotate_right(1);
        self.0[0] = Some(state);
    }
}

impl Body {
    fn new(
        id: usize,
        mass: Precision,
        x0: Precision,
        y0: Precision,
        vx0: Precision,
        vy0: Precision,
    ) -> Self {
        let current_state = State::new(x0, y0, vx0, vy0);

        Self {
            current_state,
            history: StateHistory::new(),
            mass,
            id,
        }
    }

    fn update_history(&mut self) {
        self.history.push(self.current_state.clone());
    }
}

struct Forcing {
    ax: Precision,
    ay: Precision,
    vy: Precision,
    vx: Precision
}

impl ops::Add<&Forcing> for &State {
    type Output = State;

    fn add(self, rhs: &Forcing) -> Self::Output {
        State {
            x: self.x + rhs.vx,
            y: self.y + rhs.vy,
            vx: self.vx + rhs.ax,
            vy: self.vy + rhs.ay,
        }
    }
}

impl ops::Add<&Forcing> for &Forcing {
    type Output = Forcing;

    fn add(self, rhs: &Forcing) -> Self::Output {
        Forcing {
            ax: self.ax + rhs.ax,
            ay: self.ay + rhs.ay,
            vx: self.vx + rhs.vx,
            vy: self.vy + rhs.vy,
        }
    }
}

impl ops::Mul<&Forcing> for Precision  {
    type Output = Forcing;

    fn mul(self, rhs: &Forcing) -> Self::Output {
        Self::Output {
            ax: self * rhs.ax,
            ay: self * rhs.ay,
            vx: self * rhs.vx,
            vy: self * rhs.vy,
        }
    }
}

fn forcing(state: State, thrust: i8) -> Forcing {
    let r = (state.x * state.x + state.y * state.y).sqrt();
    let v = (state.vx * state.vx + state.vy * state.vy).sqrt();

    let f = -G * MASS_EARTH / (r * r * r);

    let thrustx = thrust as Precision * THRUST * state.vx / v;
    let thrusty = thrust as Precision * THRUST * state.vy / v;

    let ax = f * state.x + thrustx; 
    let ay = f * state.y + thrusty;

    Forcing {
        ax,
        ay,
        vx: state.vx,
        vy: state.vy,
    }
}

fn rk4(state: State, thrust: i8) -> State {

    let k1 = forcing(state, thrust);
    let k2 = forcing(&state + &(0.5 * DT * &k1), thrust);
    let k3 = forcing(&state + &(0.5 * DT * &k2), thrust);
    let k4 = forcing(&state + &(DT * &k3), thrust);

    // Need to make this better without borrowing
    &state +  &(DT / 6.0 *  &(&k1 + &(&(2.0 * &k2) +  &(&(2.0 * &k3) + &k4))))
}

fn add_body(mut commands: Commands) {
    // Hardcoded for now.
    let x: Precision = 0.0;
    let y: Precision = (EARTH_RADIUS + 408000.0) as Precision; // height of ISS
    let vx: Precision = 1.1 * 7660.0; // ~ velocida de la ISS
    let vy: Precision = 0.0;

    commands.spawn(Body::new(1, 1.0, x, y, vx, vy));
}

// System that runs at each frame (I think? I don't know if each iteration is frame-based or not.)
fn system(
    mut gizmos: Gizmos,
    time: Res<Time>,
    mut query: Query<&mut Body>,
    keyboard: Res<Input<KeyCode>>,
) {
    // Draw the earth
    gizmos.circle_2d(Vec2 { x: 0.0, y: 0.0 }, EARTH_RADIUS as f32, Color::BLUE);

    for mut body in query.iter_mut() {

        let mut thrust = 0;
        let mut body_radius = 50000.0;
        if keyboard.pressed(KeyCode::Up) {
            thrust = 1;
            body_radius = 100000.0;
        }
        if keyboard.pressed(KeyCode::Down) {
            thrust = -1;
            body_radius = 100000.0;
        } 

        let mut new_state = rk4(body.current_state, thrust);
       
        body.current_state = new_state.clone();

        body.update_history();

        gizmos.circle_2d(
            Vec2 {
                x: body.current_state.x as f32,
                y: body.current_state.y as f32,
            },
            body_radius,
            Color::RED,
        );

        // draw history
        for state in body.history.0.iter() {
            if let Some(state) = state {
                gizmos.circle_2d(
                    Vec2 {
                        x: state.x as f32,
                        y: state.y as f32,
                    },
                    10000.0,
                    Color::RED,
                );
            }
        }

        // draw lookahead assuming no thrust
        for _ in 0..N_LOOKAHEAD {
            new_state = rk4(new_state, 0);
            gizmos.circle_2d(
                Vec2 {
                    x: new_state.x as f32,
                    y: new_state.y as f32,
                },
                10000.0,
                Color::GREEN,
            );
        }



    }
}

fn setup(mut commands: Commands) {
    let mut my_2d_camera_bundle = Camera2dBundle::default();

    my_2d_camera_bundle.projection.scaling_mode = ScalingMode::AutoMax {
        max_height: (EARTH_RADIUS * 6.0) as f32,
        max_width: (EARTH_RADIUS * 6.0) as f32,
    };

    commands.spawn(my_2d_camera_bundle);
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Startup, add_body)
        .add_systems(Update, system)
        .run();
}
