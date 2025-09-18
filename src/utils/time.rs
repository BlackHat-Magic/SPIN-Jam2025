use crate::*;
use std::time::Instant;

#[derive(Resource)]
pub struct Time {
    pub delta_seconds: f32,
    last_call: Instant,
}

system!(
    fn init_time(commands: commands) {
        commands.insert_resource(Time {
            delta_seconds: 0.0,
            last_call: Instant::now(),
        });
    }
);

system!(
    fn update_time(
        time: res &mut Time,
    ) {
        let Some(time) = time else {
            return;
        };
        let now = Instant::now();
        time.delta_seconds = (now - time.last_call).as_secs_f32();
        time.last_call = now;
    }
);
