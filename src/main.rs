// https://github.com/webcyou-org/wave-function-collapse-rust
// https://qiita.com/panicdragon/items/5a02d3d1470179d77ece

use bevy::{image::ImageSamplerDescriptor, prelude::*};
use bevy_aseprite_ultra::prelude::*;
use rand::prelude::SliceRandom;

const SLICE_WIDTH: u32 = 16;
const SLICE_HEIGHT: u32 = 16;
const DIM: usize = 6; // 2 x 2 のグリッド

#[derive(Debug, Clone)]
pub struct SliceMeta {
    pub rect: Rect,
    pub atlas_id: usize,
    pub pivot: Option<Vec2>,
    pub nine_patch: Option<Vec4>,
}

#[derive(Debug, Clone)]
pub struct Tile {
    slice_name: String,
    slice_meta: SliceMeta,

    pub up: Vec<usize>,
    pub right: Vec<usize>,
    pub down: Vec<usize>,
    pub left: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Cell {
    index: usize,

    pub collapsed: bool,

    /// このセルのタイルとして選択される可能性のあるタイルのインデックス
    /// 最初はすべてのタイルのインデックスで、徐々に減少していく
    pub sockets: Vec<usize>,
}

impl Cell {
    pub fn from_value(index: usize, value: usize) -> Cell {
        Cell {
            index,
            collapsed: false,
            sockets: (0..value).collect(),
        }
    }

    pub fn from_list(index: usize, value: Vec<usize>) -> Cell {
        Cell {
            index,
            collapsed: false,
            sockets: value,
        }
    }
}

#[derive(Resource)]
pub struct SourceImage(Handle<Aseprite>);

#[derive(Resource)]
pub struct WFC {
    rng: rand::rngs::StdRng,
    aseprite: Handle<Aseprite>,
    pub tiles: Vec<Tile>,
    pub grid: Vec<Cell>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: ImageSamplerDescriptor::nearest(),
        }))
        .add_plugins(AsepriteUltraPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update.run_if(resource_exists::<SourceImage>))
        .add_systems(Update, main_loop.run_if(resource_exists::<WFC>))
        .add_systems(Update, rebuild.run_if(not(resource_exists::<WFC>)))
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

            for (slice_name, slice_meta) in aseprite.slices.iter() {
                // 上辺のピクセル
                // println!("slice: {:?}", slice_name);

                if slice_meta.rect.width() as u32 != SLICE_WIDTH
                    || slice_meta.rect.height() as u32 != SLICE_HEIGHT
                {
                    error!("slice size is not 16x16");
                    continue;
                }

                // for x in 0..slice_width {
                //     let y = slice_meta.rect.min.y as u32;
                //     println!("pixel: {:?}", image.get_color_at(x, y));
                // }

                tiles.push(Tile {
                    slice_name: slice_name.clone(),
                    slice_meta: SliceMeta {
                        rect: slice_meta.rect,
                        atlas_id: slice_meta.atlas_id,
                        pivot: slice_meta.pivot,
                        nine_patch: slice_meta.nine_patch,
                    },
                    up: Vec::new(),
                    right: Vec::new(),
                    down: Vec::new(),
                    left: Vec::new(),
                });
            }

            // スライスはランダムになっているので注意
            tiles.sort_by(|a, b| a.slice_name.cmp(&b.slice_name));

            for tile in tiles.iter() {
                println!("slice: {:?}", tile.slice_name);
            }

            generating_adjacency_rules(&mut tiles, &image);

            let grid: Vec<Cell> = init_grid(aseprite.slices.len());
            let seed: [u8; 32] = [13; 32];
            // let rng = rand::SeedableRng::from_seed(seed);
            let rng = rand::SeedableRng::from_entropy();

            commands.insert_resource(WFC {
                rng,
                aseprite: source.0.clone(),
                tiles,
                grid,
            });
        }
    }
}

fn generating_adjacency_rules(mut tiles: &mut Vec<Tile>, image: &Image) {
    // 他のタイルと辺を比較し、接続可能かどうかを調べます
    let cloned = tiles.clone();
    for (current_index, current) in tiles.iter_mut().enumerate() {
        for (dest_index, dest) in cloned.iter().enumerate() {
            if current_index != dest_index {
                // 上辺
                if compare_edge(
                    &image,
                    current.slice_meta.rect.min.x as u32,
                    current.slice_meta.rect.min.y as u32,
                    dest.slice_meta.rect.min.x as u32,
                    dest.slice_meta.rect.max.y as u32 - 1,
                    1,
                    0,
                ) {
                    current.up.push(dest_index);
                }

                // 下辺
                if compare_edge(
                    &image,
                    current.slice_meta.rect.min.x as u32,
                    current.slice_meta.rect.max.y as u32 - 1,
                    dest.slice_meta.rect.min.x as u32,
                    dest.slice_meta.rect.min.y as u32,
                    1,
                    0,
                ) {
                    current.down.push(dest_index);
                }

                // 左辺

                if compare_edge(
                    &image,
                    current.slice_meta.rect.min.x as u32,
                    current.slice_meta.rect.min.y as u32,
                    dest.slice_meta.rect.max.x as u32 - 1,
                    dest.slice_meta.rect.min.y as u32,
                    0,
                    1,
                ) {
                    current.left.push(dest_index);
                }

                // 右辺
                if compare_edge(
                    &image,
                    current.slice_meta.rect.max.x as u32 - 1,
                    current.slice_meta.rect.min.y as u32,
                    dest.slice_meta.rect.min.x as u32,
                    dest.slice_meta.rect.min.y as u32,
                    0,
                    1,
                ) {
                    current.right.push(dest_index);
                }
            }
        }

        // panic!("停止");
    }
}

pub fn compare_edge(
    image: &Image,
    source_x: u32,
    source_y: u32,
    dest_x: u32,
    dest_y: u32,
    dx: u32,
    dy: u32,
) -> bool {
    for i in 0..SLICE_WIDTH {
        let dxi = dx * i;
        let dyi = dy * i;
        let source_color = image.get_color_at(source_x + dxi, source_y + dyi).unwrap();
        let dest_color = image.get_color_at(dest_x + dxi, dest_y + dyi).unwrap();
        if source_color != dest_color {
            return false;
        }
    }
    true
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

fn main_loop(mut commands: Commands, wfc: ResMut<WFC>) {
    let inner = wfc.into_inner();
    for _ in 0..100 {
        // エントロピーの低い(socketsが少ない、最も選択肢の少ない)セルを選択
        let mut low_entropy_grid = pick_cell_with_least_entropy(&mut inner.grid);

        if low_entropy_grid.is_empty() {
            for cell in inner.grid.iter() {
                commands.spawn((
                    AseSpriteSlice {
                        aseprite: inner.aseprite.clone(),
                        name: inner.tiles[cell.sockets[0]].slice_name.clone(),
                    },
                    Transform::from_translation(Vec3::new(
                        (cell.index % DIM) as f32 * SLICE_WIDTH as f32,
                        (cell.index / DIM) as f32 * SLICE_HEIGHT as f32 * -1.0,
                        0.0,
                    )),
                ));
            }

            commands.remove_resource::<WFC>();

            return;
        }

        // 候補からひとつをランダムに選択
        if !random_selection_of_sockets(&mut inner.rng, &mut low_entropy_grid) {
            // 候補が見つからない場合は最初からやり直し
            inner.grid = init_grid(inner.tiles.len());
            // warn!("restart");
            return;
        }

        wave_collapse(&mut inner.grid, &inner.tiles);
    }
}

pub fn pick_cell_with_least_entropy(grid: &mut Vec<Cell>) -> Vec<&mut Cell> {
    let mut grid_copy: Vec<&mut Cell> = Vec::new();

    for cell in grid.iter_mut() {
        if !cell.collapsed {
            grid_copy.push(cell);
        }
    }
    if grid_copy.is_empty() {
        return Vec::new();
    }
    grid_copy.sort_by_key(|cell| cell.sockets.len());

    let len = grid_copy[0].sockets.len();
    let stop_index = grid_copy
        .iter()
        .position(|cell| cell.sockets.len() > len)
        .unwrap_or(grid_copy.len());

    grid_copy.truncate(stop_index);
    grid_copy
}

pub fn random_selection_of_sockets(
    mut rng: &mut rand::rngs::StdRng,
    grid_target: &mut Vec<&mut Cell>,
) -> bool {
    if let Some(cell) = grid_target.choose_mut(&mut rng) {
        (*cell).collapsed = true;

        if cell.sockets.is_empty() {
            return false;
        }
        if let Some(&pick) = cell.sockets.choose(&mut rng) {
            cell.sockets = vec![pick];
            true
        } else {
            false
        }
    } else {
        false
    }
}

pub fn wave_collapse(grid: &mut Vec<Cell>, tiles: &Vec<Tile>) {
    let mut next_grid: Vec<Option<Cell>> = vec![None; DIM * DIM];

    for j in 0..DIM {
        for i in 0..DIM {
            let index = i + j * DIM;

            if grid[index].collapsed {
                next_grid[index] = Some(grid[index].clone());
            } else {
                let mut sockets: Vec<usize> = (0..tiles.len()).collect();
                // Look up
                if j > 0 {
                    cell_collapse(&mut grid[i + (j - 1) * DIM], "down", &mut sockets, tiles);
                }
                // Look right
                if i < DIM - 1 {
                    cell_collapse(&mut grid[i + 1 + j * DIM], "left", &mut sockets, tiles);
                }
                // Look down
                if j < DIM - 1 {
                    cell_collapse(&mut grid[i + (j + 1) * DIM], "up", &mut sockets, tiles);
                }
                // Look left
                if i > 0 {
                    cell_collapse(&mut grid[i - 1 + j * DIM], "right", &mut sockets, tiles);
                }

                next_grid[index] = Some(Cell::from_list(index, sockets));
            }
        }
    }
    grid.clear();
    grid.extend(next_grid.into_iter().filter_map(|cell| cell));
}

/// セルのsocketsのうち、接続不可能なものを削除します
fn cell_collapse(cell: &Cell, direction: &str, sockets: &mut Vec<usize>, tiles: &[Tile]) {
    let valid_sockets = get_valid_sockets(cell, direction, tiles);
    sockets.retain(|socket| valid_sockets.contains(socket));
}

fn get_valid_sockets(cell: &Cell, direction: &str, tiles: &[Tile]) -> Vec<usize> {
    let mut valid_sockets = Vec::new();

    for &socket in &cell.sockets {
        let tile = &tiles[socket];

        let valid = match direction {
            "up" => tile.up.clone(),
            "right" => tile.right.clone(),
            "down" => tile.down.clone(),
            "left" => tile.left.clone(),
            _ => Vec::new(),
        };

        valid_sockets.extend(valid);
    }
    valid_sockets
}
