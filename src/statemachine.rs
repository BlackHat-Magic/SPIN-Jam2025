use std::sync::Arc;

use glam::*;

pub use ecs::*;
pub use networking::*;

pub mod physics;
pub mod render;
pub mod utils;

pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
pub use render::*;
// pub use utils::time::*;
pub use utils::*;

//Brandon's Enemy AI
pub use rand::prelude::*;
// pub use utils::time;
use std::time::{Duration};
use std::thread;

//within a system
//Should probably not be returning it
//Just need to assign it and run with it.

#[derive(Component)]
#[derive(Debug)]
#[derive(Copy, Clone)]
#[derive(PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    None,
}

#[derive(PartialEq)]
pub enum Movement {
    Idle,
    Directional,
    Walking,
    Both,
}

pub struct StateMachine {
    pub pos: Vec3,
    pub scale: Vec3,
    pub direction: Direction, //directional facing
    pub facings: Vec<Direction>, //This shoud limit the facing to only one, two
    pub movement: Movement, //Indicates what kind of movement they are able to
    pub mobility: Vec<Direction>,
    pub boundaries: Vec2,
    pub start: Vec2,
}

//TODO:
//Build a struct builder which allows for the setting of diffeent
//positions, directional facings, movement (Idle, moving)


impl Default for StateMachine {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            direction: Direction::None,
            facings: Vec::new(),
            movement: Movement::Idle,
            mobility: Vec::new(),
            boundaries: Vec2 {
                x: 0.0,
                y: 0.0,
            },
            start: Vec2 {
                x: 0.0,
                y: 0.0,
            },
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
        let len = self.facings.len();
        let value = rng.random_range(0..len);
        self.direction = self.facings[value];
        println!("Direction the enemy is currently facing: {:?}", self.direction);
    }

    pub fn movement_chance(&mut self) {
        let mut rng = rand::rng();
        let len = self.mobility.len();
        let value = rng.random_range(0..len);
        if self.mobility[value] == Direction::Up {
            if (self.start.y + 1.0).abs() <= self.boundaries.y {
                self.start.y += 1.0;
            }
        }
        else if self.mobility[value] == Direction::Down {
            if (self.start.y - 1.0).abs() <= self.boundaries.y {
                self.start.y -= -1.0;
            }
        }
        else if self.mobility[value] == Direction::Left {
            if (self.start.x - 1.0).abs() <= self.boundaries.x {
                self.start.x -= 1.0;
            }
        }
        else if self.mobility[value] == Direction::Right {
            if (self.start.x + 1.0).abs() <= self.boundaries.x {
                self.start.x += 1.0;
            }
        }
    }

    //Sort of like a FNAF movement opportunity kind of deal
    //If the random variable is equal to 1, the enemy switches direction
    //
    //Potential Enhancements:
    //Some enemies move and some enemies don't move their head
    //(Could be assigned in the default variable)
    pub fn enemy_direction_opportunity(&mut self) {
        let mut rng = rand::rng();
        if rng.random_range(0..4) <= 1 {
            println!("Direction change occuring");
            self.direction_change();
        }
        else {
            println!("Direction change hasn't occured");
        }
    }

    pub fn enemy_movement_opportunity(&mut self){
        let mut rng = rand::rng();
        if rng.random_range(0..4) <= 1 {
            println!("Movement occuring");
            self.direction_change();
        } else {
            println!("Movement hasn't occured")
        }
    }
}

fn main() {
    let mut enemy_ai = StateMachine::default();
    let mut rng = rand::rng();
    //Loops for every 2 seconds. This probably isn't the most efficient.
    //I had tried other ideas in previous pushes.
    let both = Movement::Both;
    let idle = Movement::Idle;
    let directional = Movement::Directional;
    let walking = Movement::Walking;
    if enemy_ai.movement != idle {
        loop {
            if rng.random_range(0..2) == 0 { //Directional opportunity
                if both == enemy_ai.movement || directional == enemy_ai.movement {
                    enemy_ai.enemy_direction_opportunity();
                }
            } else { //Movement opportunity
                if both == enemy_ai.movement || walking == enemy_ai.movement {
                    enemy_ai.enemy_movement_opportunity();
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    }
}