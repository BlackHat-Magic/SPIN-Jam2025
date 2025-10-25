use std::sync::Arc;

use glam::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub use ecs::*;
pub use networking::*;

pub mod physics;
pub mod render;
pub mod utils;

pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
pub use render::*;
use utils::input::Input;
// pub use utils::time::*;
pub use utils::*;

//Brandon's Enemy AI
pub use rand::prelude::*;
// pub use utils::time;
use std::time::{Instant, Duration};
use std::thread;

//within a system
//Should probably not be returning it
//Just need to assign it and run with it.

#[derive(Component)]
#[derive(Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct StateMachine {
    pub pos: Vec3,
    pub scale: Vec3,
    pub direction: Direction, //directional facing
}


impl Default for StateMachine {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            direction: Direction::Up,
        }
    }
}

impl StateMachine {
    //Random variable is made from 0 to 3
    //This determines which direction the enemy looks at
    //
    //Possible enhancements:
    //More than just a 4-way directional movement, could be 8-directions
    pub fn direction_change(&mut self) {
        let mut rng = rand::rng();
        let value = rng.random_range(0..4);
        if value == 0 {
            self.direction = Direction::Up;
        }
        if value == 1 {
            self.direction = Direction::Down;
        }
        if value == 2 {
            self.direction = Direction::Left;
        }
        if value == 3 {
            self.direction = Direction::Right;
        }
        if value > 3 {
            self.direction = Direction::Up;
        }
        println!("Direction the enemy is currently facing: {:?}", self.direction);
    }

    //Sort of like a FNAF movement opportunity kind of deal
    //If the random variable is equal to 1, the enemy switches direction
    //
    //Potential Enhancements:
    //Some enemies move and some enemies don't move their head
    //(Could be assigned in the default variable)
    pub fn enemy_movement_opportunity(&mut self) {
        let mut rng = rand::rng();
        if rng.random_range(0..10) == 0 {
            println!("Direction change occuring");
            self.direction_change();
        }
        else {
            println!("Direction change hasn't occured");
        }
    }

    //Trying to implement a waiting time of 3 seconds for the enemies
    //Waiting time on head movement can be randomized, but that may add too much rng
    //So it may just be better to just stick with a set movement.
    // fn enemy_waiting(
    //     &mut self,
    // ) {
    //     self.wait += delta_seconds;
    //     if self.wait > self.idle {
    //         self.enemy_movement_opportunity();
    //         self.wait = 0;
    //     }
    // }
}

fn main() {
    let mut direction = StateMachine::default();

    loop {
        direction.enemy_movement_opportunity();
        thread::sleep(Duration::from_secs(2));
    }
}