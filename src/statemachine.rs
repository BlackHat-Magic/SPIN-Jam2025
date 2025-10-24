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
            rand: 0,
            rot: Direction::Up,
        }
    }
}

impl StateMachine {
    pub fn direction_change(&mut self) -> &Self {
        match self.rand {
            0 => {
                self.rot = Direction::Up;
            }
            1 => {
                self.rot = Direction::Down;
            }
            2 => {
                self.rot = Direction::Left;
            }
            3 => {
                self.rot = Direction::Right;
            }
            _ => {}
        }
        self
    }
}

fn enemy_movement() {

}

fn main() {

}