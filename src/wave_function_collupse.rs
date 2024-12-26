// https://github.com/webcyou-org/wave-function-collapse-rust
// https://qiita.com/panicdragon/items/5a02d3d1470179d77ece

use bevy::prelude::*;
use rand::prelude::SliceRandom;

pub const SLICE_WIDTH: u32 = 16;
pub const SLICE_HEIGHT: u32 = 16;
pub const DIM: usize = 8;

#[derive(Debug, Clone)]
pub struct Tile {
    pub slice_name: String,
    pub rect: Rect,

    pub up: Vec<usize>,
    pub right: Vec<usize>,
    pub down: Vec<usize>,
    pub left: Vec<usize>,
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

pub fn generating_adjacency_rules(tiles: &mut Vec<Tile>, image: &Image) {
    // 他のタイルと辺を比較し、接続可能かどうかを調べます
    let cloned = tiles.clone();
    for (current_index, current) in tiles.iter_mut().enumerate() {
        for (dest_index, dest) in cloned.iter().enumerate() {
            if current_index != dest_index {
                // 上辺
                if compare_edge(
                    &image,
                    current.rect.min.x as u32,
                    current.rect.min.y as u32,
                    dest.rect.min.x as u32,
                    dest.rect.max.y as u32 - 1,
                    1,
                    0,
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
                ) {
                    current.right.push(dest_index);
                }
            }
        }
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
