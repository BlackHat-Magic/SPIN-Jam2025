#[derive(Component)]

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct StateMachine {
    pub pos: Vec3,
    pub scale: Vec3,
    pub rand: i32,
    pub rot: Quat,
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
            rot: Direction::Up,
        }
    }

    fn direction_change(&mut self, rand: i32) -> &self {
        if self.rand == 0 {
            self.rot = Direction::Up;
        }
        else if self.rand == 1 {
            self.rot = Direction::Down;
        }
        else if self.rand == 2 {
            self.rot = Direction::Left;
        }
        else if self.rand == 3 {
            self.rot = Direction::Right;
        }  
    }
}

fn enemy_movement() {

}

fn main() {

}