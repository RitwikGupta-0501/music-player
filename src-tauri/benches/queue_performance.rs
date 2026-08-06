use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use echo_desktop_lib::queue::{QueueState, QueueTrack, TrackSourceInfo, RepeatMode};

fn make_track(id: &str) -> QueueTrack {
    QueueTrack {
        instance_id: id.to_string(),
        title: format!("Track {}", id),
        artist: Some("Test Artist".to_string()),
        track_number: Some(1),
        source: TrackSourceInfo::Local {
            track_id: id.parse().unwrap_or(1),
            file_path: format!("/path/to/{}.mp3", id),
            album_id: Some(1),
        },
    }
}

fn benchmark_set_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_queue");

    for size in [100, 1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tracks: Vec<QueueTrack> = (0..size)
                .map(|i| make_track(&i.to_string()))
                .collect();

            b.iter(|| {
                let mut queue = QueueState::new();
                queue.set_queue(black_box(tracks.clone()), 0).unwrap();
            });
        });
    }

    group.finish();
}

fn benchmark_next_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("next");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("normal", size), size, |b, &size| {
            let mut queue = QueueState::new();
            let tracks: Vec<QueueTrack> = (0..size)
                .map(|i| make_track(&i.to_string()))
                .collect();
            queue.set_queue(tracks, 0).unwrap();
            queue.repeat_mode = RepeatMode::All;

            b.iter(|| {
                let mut q = queue.clone();
                q.next().ok();
            });
        });

        group.bench_with_input(BenchmarkId::new("shuffle", size), size, |b, &size| {
            let mut queue = QueueState::new();
            let tracks: Vec<QueueTrack> = (0..size)
                .map(|i| make_track(&i.to_string()))
                .collect();
            queue.set_queue(tracks, 0).unwrap();
            queue.set_shuffle(true).ok();

            b.iter(|| {
                let mut q = queue.clone();
                q.next().ok();
            });
        });
    }

    group.finish();
}

fn benchmark_reorder(c: &mut Criterion) {
    let mut group = c.benchmark_group("reorder");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tracks: Vec<QueueTrack> = (0..size)
                .map(|i| make_track(&i.to_string()))
                .collect();

            b.iter(|| {
                let mut queue = QueueState::new();
                queue.set_queue(black_box(tracks.clone()), 0).unwrap();

                // Reorder from 0 to middle
                let target = size / 2;
                queue.reorder(0, target).ok();
            });
        });
    }

    group.finish();
}

fn benchmark_shuffle_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle");

    for size in [100, 1000, 10000, 50000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tracks: Vec<QueueTrack> = (0..size)
                .map(|i| make_track(&i.to_string()))
                .collect();

            b.iter(|| {
                let mut queue = QueueState::new();
                queue.set_queue(black_box(tracks.clone()), 0).unwrap();
                queue.regenerate_shuffle_order().ok();
            });
        });
    }

    group.finish();
}

fn benchmark_jump_to_track(c: &mut Criterion) {
    let mut group = c.benchmark_group("jump");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let tracks: Vec<QueueTrack> = (0..size)
                .map(|i| make_track(&i.to_string()))
                .collect();

            b.iter(|| {
                let mut queue = QueueState::new();
                queue.set_queue(black_box(tracks.clone()), 0).unwrap();

                // Jump to middle track
                let target_id = (size / 2).to_string();
                queue.jump_to_instance_id(target_id).ok();
            });
        });
    }

    group.finish();
}

fn benchmark_sequential_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential");
    group.sample_size(10);  // Reduce sample size for slower tests

    group.bench_function("10_skips_100_items", |b| {
        let tracks: Vec<QueueTrack> = (0..100)
            .map(|i| make_track(&i.to_string()))
            .collect();

        b.iter(|| {
            let mut queue = QueueState::new();
            queue.set_queue(black_box(tracks.clone()), 0).unwrap();
            queue.repeat_mode = RepeatMode::All;

            for _ in 0..10 {
                queue.next().ok();
            }
        });
    });

    group.bench_function("100_skips_10k_items", |b| {
        let tracks: Vec<QueueTrack> = (0..10000)
            .map(|i| make_track(&i.to_string()))
            .collect();

        b.iter(|| {
            let mut queue = QueueState::new();
            queue.set_queue(black_box(tracks.clone()), 0).unwrap();
            queue.repeat_mode = RepeatMode::All;

            for _ in 0..100 {
                queue.next().ok();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_set_queue,
    benchmark_next_operation,
    benchmark_reorder,
    benchmark_shuffle_generation,
    benchmark_jump_to_track,
    benchmark_sequential_operations
);

criterion_main!(benches);
