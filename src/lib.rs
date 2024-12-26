// https://github.com/webcyou-org/wave-function-collapse-rust
// https://qiita.com/panicdragon/items/5a02d3d1470179d77ece

use bevy::prelude::*;
use bevy_aseprite_ultra::prelude::{AseSpriteSlice, Aseprite};
use rand::{prelude::SliceRandom, rngs::StdRng};

#[derive(Debug, Clone)]
pub struct Tile {
    pub slice_name: String,
    pub rect: Rect,

    pub up: Vec<usize>,
    pub right: Vec<usize>,
    pub down: Vec<usize>,
    pub left: Vec<usize>,
}

impl Tile {
    pub fn new(slice_name: String, rect: Rect) -> Tile {
        Tile {
            slice_name,
            rect,
            up: Vec::new(),
            right: Vec::new(),
            down: Vec::new(),
            left: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct Tileset {
    pub tiles: Vec<Tile>,
    pub tile_size: u32,
}

#[derive(Clone)]
pub struct Grid {
    pub tileset: Tileset,
    pub cells: Vec<Cell>,
    pub dimension: usize,
}

impl Tileset {
    /// Asepriteファイルと画像からタイルセットを生成します
    /// スライスのサイズはすべて統一されている必要があります
    pub fn new(aseprite: &Aseprite, image: &Image) -> Self {
        // ソースの画像の読み込みが完了したらタイルを初期化
        let mut tiles: Vec<Tile> = Vec::new();

        let tile_size = aseprite.slices.iter().next().unwrap().1.rect.width() as u32;

        // Asepriteファイルからすべてのスライスを取得し、タイルに変換します
        for (slice_name, slice_meta) in aseprite.slices.iter() {
            if slice_meta.rect.width() as u32 != tile_size
                || slice_meta.rect.height() as u32 != tile_size
            {
                error!("slice size is not {}x{}", tile_size, tile_size);
            }
            tiles.push(Tile::new(slice_name.clone(), slice_meta.rect));
        }

        // スライスはランダムな順序になっているので注意
        // 通路のない空白のタイルが0番目になるようにソートします
        tiles.sort_by(|a, b| a.slice_name.cmp(&b.slice_name));

        let mut tileset = Tileset { tiles, tile_size };

        // 隣接関係を生成します
        generating_adjacency_rules(&mut tileset, &image, tile_size);

        tileset
    }
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub index: usize,

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

impl Tileset {
    pub fn create_grid(&self, dimension: usize) -> Grid {
        let cells = (0..dimension * dimension)
            .map(|index| Cell::from_value(index, self.tiles.len()))
            .collect();
        Grid {
            tileset: self.clone(),
            cells,
            dimension,
        }
    }
}

/// 他のタイルと辺のピクセルを比較し、
/// 完全に一致した場合は接続可能としてタイル四方のソケットに追加します
pub fn generating_adjacency_rules(tiles: &mut Tileset, image: &Image, tile_size: u32) {
    let cloned = tiles.clone();
    for current in tiles.tiles.iter_mut() {
        for (dest_index, dest) in cloned.tiles.iter().enumerate() {
            // 上辺
            if compare_edge(
                &image,
                current.rect.min.x as u32,
                current.rect.min.y as u32,
                dest.rect.min.x as u32,
                dest.rect.max.y as u32 - 1,
                1,
                0,
                tile_size,
            ) {
                current.up.push(dest_index);
            }

            // 下辺
            if compare_edge(
                &image,
                current.rect.min.x as u32,
                current.rect.max.y as u32 - 1,
                dest.rect.min.x as u32,
                dest.rect.min.y as u32,
                1,
                0,
                tile_size,
            ) {
                current.down.push(dest_index);
            }

            // 左辺

            if compare_edge(
                &image,
                current.rect.min.x as u32,
                current.rect.min.y as u32,
                dest.rect.max.x as u32 - 1,
                dest.rect.min.y as u32,
                0,
                1,
                tile_size,
            ) {
                current.left.push(dest_index);
            }

            // 右辺
            if compare_edge(
                &image,
                current.rect.max.x as u32 - 1,
                current.rect.min.y as u32,
                dest.rect.min.x as u32,
                dest.rect.min.y as u32,
                0,
                1,
                tile_size,
            ) {
                current.right.push(dest_index);
            }
        }
    }
}

fn compare_edge(
    image: &Image,
    source_x: u32,
    source_y: u32,
    dest_x: u32,
    dest_y: u32,
    dx: u32,
    dy: u32,
    tile_size: u32,
) -> bool {
    for i in 0..tile_size {
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

fn pick_cell_with_least_entropy(cells: &mut Vec<Cell>) -> Vec<&mut Cell> {
    let mut grid_copy: Vec<&mut Cell> = Vec::new();

    for cell in cells.iter_mut() {
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

fn random_selection_of_sockets(
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

fn wave_collapse(cells: &mut Vec<Cell>, dimension: usize, tileset: &Tileset) {
    let mut next_grid: Vec<Option<Cell>> = vec![None; dimension * dimension];

    for j in 0..dimension {
        for i in 0..dimension {
            let index = i + j * dimension;

            if cells[index].collapsed {
                next_grid[index] = Some(cells[index].clone());
            } else {
                let mut sockets: Vec<usize> = (0..tileset.tiles.len()).collect();
                // Look up
                if j > 0 {
                    cell_collapse(
                        &mut cells[i + (j - 1) * dimension],
                        "down",
                        &mut sockets,
                        &tileset,
                    );
                }
                // Look right
                if i < dimension - 1 {
                    cell_collapse(
                        &mut cells[i + 1 + j * dimension],
                        "left",
                        &mut sockets,
                        &tileset,
                    );
                }
                // Look down
                if j < dimension - 1 {
                    cell_collapse(
                        &mut cells[i + (j + 1) * dimension],
                        "up",
                        &mut sockets,
                        &tileset,
                    );
                }
                // Look left
                if i > 0 {
                    cell_collapse(
                        &mut cells[i - 1 + j * dimension],
                        "right",
                        &mut sockets,
                        &tileset,
                    );
                }

                next_grid[index] = Some(Cell::from_list(index, sockets));
            }
        }
    }
    cells.clear();
    cells.extend(next_grid.into_iter().filter_map(|cell| cell));
}

/// セルのsocketsのうち、接続不可能なものを削除します
fn cell_collapse(cell: &Cell, direction: &str, sockets: &mut Vec<usize>, tiles: &Tileset) {
    let valid_sockets = get_valid_sockets(cell, direction, tiles);
    sockets.retain(|socket| valid_sockets.contains(socket));
}

fn get_valid_sockets(cell: &Cell, direction: &str, tiles: &Tileset) -> Vec<usize> {
    let mut valid_sockets = Vec::new();

    for &socket in &cell.sockets {
        let tile = &tiles.tiles[socket];

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

impl Grid {
    pub fn new(aseprite: &Aseprite, image: &Image, dimension: usize) -> Self {
        Tileset::new(aseprite, image).create_grid(dimension)
    }

    pub fn collapse_with(&mut self, mut rng: &mut rand::rngs::StdRng) {
        let mut cells = self.cells.clone();

        loop {
            // エントロピーの低い(socketsが少ない、最も選択肢の少ない)セルを選択
            let mut low_entropy_grid = pick_cell_with_least_entropy(&mut cells);

            if low_entropy_grid.is_empty() {
                break;
            }

            // 候補からひとつをランダムに選択
            if !random_selection_of_sockets(&mut rng, &mut low_entropy_grid) {
                // 候補が見つからない場合は最初からやり直し
                cells = self.cells.clone();
                // warn!("restart");
                continue;
            }

            wave_collapse(&mut cells, self.dimension, &self.tileset);
        }

        self.cells = cells;
    }

    pub fn collapse(&mut self) {
        let mut rng: StdRng = rand::SeedableRng::from_entropy();
        self.collapse_with(&mut rng);
    }

    pub fn spawn(&self, commands: &mut Commands, aseprite: &Handle<Aseprite>) {
        for cell in self.cells.iter() {
            commands.spawn((
                AseSpriteSlice {
                    aseprite: aseprite.clone(),
                    name: self.tileset.tiles[cell.sockets[0]].slice_name.clone(),
                },
                Transform::from_translation(Vec3::new(
                    (cell.index % self.dimension) as f32 * self.tileset.tile_size as f32,
                    (cell.index / self.dimension) as f32 * self.tileset.tile_size as f32 * -1.0,
                    0.0,
                )),
            ));
        }
    }

    pub fn spawn_with_world(&self, commands: &mut World, aseprite: &Handle<Aseprite>) {
        for cell in self.cells.iter() {
            commands.spawn((
                AseSpriteSlice {
                    aseprite: aseprite.clone(),
                    name: self.tileset.tiles[cell.sockets[0]].slice_name.clone(),
                },
                Transform::from_translation(Vec3::new(
                    (cell.index % self.dimension) as f32 * self.tileset.tile_size as f32,
                    (cell.index / self.dimension) as f32 * self.tileset.tile_size as f32 * -1.0,
                    0.0,
                )),
            ));
        }
    }
}
