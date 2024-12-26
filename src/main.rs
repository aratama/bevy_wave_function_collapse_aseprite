// https://github.com/webcyou-org/wave-function-collapse-rust
// https://qiita.com/panicdragon/items/5a02d3d1470179d77ece

mod wave_function_collupse;

use bevy::{
    ecs::world::CommandQueue,
    image::ImageSamplerDescriptor,
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bevy_aseprite_ultra::prelude::*;
use wave_function_collupse::{
    generating_adjacency_rules, pick_cell_with_least_entropy, random_selection_of_sockets,
    wave_collapse, Cell, Tile, DIM, SLICE_HEIGHT, SLICE_WIDTH,
};

#[derive(Resource)]
pub struct SourceImage(Handle<Aseprite>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: ImageSamplerDescriptor::nearest(),
        }))
        .add_plugins(AsepriteUltraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update.run_if(resource_exists::<SourceImage>))
        .add_systems(Update, rebuild)
        .add_systems(Update, handle_tasks)
        .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(
            SLICE_WIDTH as f32 * DIM as f32 / 2.0,
            SLICE_HEIGHT as f32 * DIM as f32 / -2.0,
            0.0,
        )
        .with_scale(Vec3::splat(0.4)),
    ));
    commands.insert_resource(SourceImage(server.load("image.aseprite")));
}

fn rebuild(
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    server: Res<AssetServer>,
    query: Query<Entity, With<AseSpriteSlice>>,
) {
    if mouse.just_pressed(MouseButton::Left) {
        commands.insert_resource(SourceImage(server.load("image.aseprite")));
        for entity in query.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Component)]
struct ComputeTransform(Task<CommandQueue>);

fn update(
    mut commands: Commands,
    aseprites: Res<Assets<Aseprite>>,
    source: Res<SourceImage>,
    images: Res<Assets<Image>>,
) {
    if let Some(aseprite) = aseprites.get(source.0.id()) {
        if let Some(image) = images.get(aseprite.atlas_image.id()) {
            // ソースの画像の読み込みが完了したらグリッドを初期化
            commands.remove_resource::<SourceImage>();

            let mut tiles: Vec<Tile> = Vec::new();

            let aseplite_cloned = source.0.clone();

            for (slice_name, slice_meta) in aseprite.slices.iter() {
                if slice_meta.rect.width() as u32 != SLICE_WIDTH
                    || slice_meta.rect.height() as u32 != SLICE_HEIGHT
                {
                    error!("slice size is not 16x16");
                    continue;
                }

                tiles.push(Tile {
                    slice_name: slice_name.clone(),
                    rect: slice_meta.rect,
                    up: Vec::new(),
                    right: Vec::new(),
                    down: Vec::new(),
                    left: Vec::new(),
                });
            }

            // スライスはランダムになっているので注意
            tiles.sort_by(|a, b| a.slice_name.cmp(&b.slice_name));

            generating_adjacency_rules(&mut tiles, &image);

            let mut grid: Vec<Cell> = init_grid(aseprite.slices.len());

            // let seed: [u8; 32] = [13; 32];
            // let rng = rand::SeedableRng::from_seed(seed);
            let mut rng = rand::SeedableRng::from_entropy();

            let thread_pool = AsyncComputeTaskPool::get();
            let entity = commands.spawn_empty().id();
            let task: Task<CommandQueue> = thread_pool.spawn(async move {
                loop {
                    // エントロピーの低い(socketsが少ない、最も選択肢の少ない)セルを選択
                    let mut low_entropy_grid = pick_cell_with_least_entropy(&mut grid);

                    if low_entropy_grid.is_empty() {
                        break;
                    }

                    // 候補からひとつをランダムに選択
                    if !random_selection_of_sockets(&mut rng, &mut low_entropy_grid) {
                        // 候補が見つからない場合は最初からやり直し
                        grid = init_grid(tiles.len());
                        // warn!("restart");
                        continue;
                    }

                    wave_collapse(&mut grid, &tiles);
                }

                let mut command_queue = CommandQueue::default();
                command_queue.push(move |world: &mut World| {
                    world.entity_mut(entity).remove::<ComputeTransform>();

                    for cell in grid.iter() {
                        world.spawn((
                            AseSpriteSlice {
                                aseprite: aseplite_cloned.clone(),
                                name: tiles[cell.sockets[0]].slice_name.clone(),
                            },
                            Transform::from_translation(Vec3::new(
                                (cell.index % DIM) as f32 * SLICE_WIDTH as f32,
                                (cell.index / DIM) as f32 * SLICE_HEIGHT as f32 * -1.0,
                                0.0,
                            )),
                        ));
                    }
                });
                command_queue
            });
            commands.entity(entity).insert(ComputeTransform(task));
        }
    }
}

fn handle_tasks(mut commands: Commands, mut transform_tasks: Query<&mut ComputeTransform>) {
    for mut task in transform_tasks.iter_mut() {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
        }
    }
}

fn init_grid(length: usize) -> Vec<Cell> {
    let mut grid: Vec<Cell> = (0..DIM * DIM)
        .map(|index| Cell::from_value(index, length))
        .collect();

    // for cell in grid.iter_mut() {
    //     let x = cell.index % DIM;
    //     let y = cell.index / DIM;
    //     if x == 0 {
    //         cell.collapsed = true;
    //         cell.sockets = vec![0];
    //     }
    // }

    grid
}
