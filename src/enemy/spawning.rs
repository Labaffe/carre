fn spawn_green_ufos_oneshot(
    mut commands: Commands,
    mut difficulty: ResMut<Difficulty>,
    frames: Res<GreenUFOFrames>,
    windows: Query<&Window>,
) {
    let Some(pos) = difficulty
        .spawn_requests
        .iter()
        .position(|(n, _, _)| *n == "green_ufo")
    else {
        return;
    };
    let (_name, count, spawn_pos) = difficulty.spawn_requests.remove(pos);

    let window = windows.single();
    for _ in 0..count {
        spawn_one(&mut commands, &frames, window, spawn_pos);
    }
}
fn spawn_green_ufos(
    mut commands: Commands,
    time: Res<Time>,
    mut spawner: ResMut<GreenUFOSpawner>,
    difficulty: Res<Difficulty>,
    frames: Res<GreenUFOFrames>,
    windows: Query<&Window>,
) {
    let Some(&(wave_size, target_interval, spawn_pos)) = difficulty.active_spawners.get("green_ufo")
    else {
        return;
    };

    if (spawner.timer.duration().as_secs_f32() - target_interval).abs() > 0.01 {
        spawner
            .timer
            .set_duration(std::time::Duration::from_secs_f32(target_interval));
    }

    spawner.timer.tick(time.delta());
    if !spawner.timer.just_finished() {
        return;
    }

    let window = windows.single();
    for _ in 0..wave_size {
        spawn_one(&mut commands, &frames, window, spawn_pos);
    }
}

