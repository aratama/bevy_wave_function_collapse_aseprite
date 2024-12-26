// https://github.com/webcyou-org/wave-function-collapse-rust
// https://qiita.com/panicdragon/items/5a02d3d1470179d77ece

use bevy::{
    ecs::world::CommandQueue,
    image::ImageSamplerDescriptor,
    prelude::*,
    tasks::{block_on, poll_once, AsyncComputeTaskPool, Task},
};
use bevy_aseprite_ultra::prelude::*;
use bevy_wave_function_collapse_aseprite::Grid;
use rand::{rngs::StdRng, Rng};

/// 生成するグリッドの縦横のセル数
const DIMENSION: usize = 16;

/// タイルの縦横のピクセルサイズ
const TILE_SIZE: u32 = 16;

#[derive(Resource)]
pub struct SourceImage(Handle<Aseprite>);

#[derive(Component)]
struct WaveFunctionCollapseTask(Task<CommandQueue>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: ImageSamplerDescriptor::nearest(),
        }))
        .add_plugins(AsepriteUltraPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            run_wave_function_collupse_task.run_if(resource_exists::<SourceImage>),
        )
        .add_systems(Update, rebuild)
        .add_systems(Update, handle_tasks)
        .run();
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Transform::from_xyz(
            TILE_SIZE as f32 * DIMENSION as f32 / 2.0,
            TILE_SIZE as f32 * DIMENSION as f32 / -2.0,
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

fn run_wave_function_collupse_task(
    mut commands: Commands,
    source: Res<SourceImage>,
    aseprites: Res<Assets<Aseprite>>,
    images: Res<Assets<Image>>,
) {
    if let Some(aseprite) = aseprites.get(source.0.id()) {
        if let Some(image) = images.get(aseprite.atlas_image.id()) {
            commands.remove_resource::<SourceImage>();

            // ソースの画像の読み込みが完了したらタイルセットを初期化
            // 最初はすべてのセルがすべてのソケットを持っている状態(どのセルもどのタイルへと崩壊する可能性がある)です
            let mut grid: Grid = Grid::new(&aseprite, &image, DIMENSION);

            // タイルの生成までは Aseprite のインスタンスを参照するので、
            // ライフタイムの問題により非同期タスクの内部では実行できません
            // ここから非同期タスクを開始します
            let aseplite_cloned = source.0.clone();
            let thread_pool = AsyncComputeTaskPool::get();
            let entity = commands.spawn_empty().id();
            let task: Task<CommandQueue> = thread_pool.spawn(async move {
                // 結果を再現可能にするにはシードを指定して乱数生成器を初期化します
                // let seed: [u8; 32] = [42; 32];
                // let mut rng = rand::SeedableRng::from_seed(seed);
                let mut rng: StdRng = rand::SeedableRng::from_entropy();

                // 行き止まりの通路が生成されないように、外周のセルを空白タイルにします
                // また、通路や部屋の密度が高くなりすぎないように、ランダムに空白タイルを設定します
                for cell in grid.cells.iter_mut() {
                    let x = cell.index % DIMENSION;
                    let y = cell.index / DIMENSION;
                    if x == 0
                        || y == 0
                        || x == DIMENSION - 1
                        || y == DIMENSION - 1
                        || rng.gen::<u32>() % 4 == 0
                    {
                        cell.collapsed = true;
                        cell.sockets = vec![0];
                    }
                }

                // 波動関数の崩壊を実行します
                // サイズによってはこれに数秒かかる場合があります
                grid.collapse_with(&mut rng);

                // 崩壊が完了したらスプライトを生成して結果を表示します
                let mut command_queue = CommandQueue::default();
                command_queue.push(move |mut world: &mut World| {
                    // 完了したタスクは忘れずに削除しておきます
                    world.entity_mut(entity).despawn_recursive();

                    grid.spawn_with_world(&mut world, &aseplite_cloned);
                });
                command_queue
            });
            commands
                .entity(entity)
                .insert(WaveFunctionCollapseTask(task));
        }
    }
}

fn handle_tasks(mut commands: Commands, mut transform_tasks: Query<&mut WaveFunctionCollapseTask>) {
    for mut task in transform_tasks.iter_mut() {
        if let Some(mut commands_queue) = block_on(poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
        }
    }
}
