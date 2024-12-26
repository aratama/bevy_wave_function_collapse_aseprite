# bevy_wave_function_collapse_aseprite

Create tileset with Aseprite, run Wave Function Collapse, and display tiles with Bevy Engine.

![screenshot](docs/screenshot.png)

# Usage

#### 1. Create Tileset Image with Aseprite

![aseprite screenshot](docs/aseprite.png)

#### 2. Create Grid

```rust
let mut grid = Grid::new(&aseprite, &image, DIMENSION);
```

#### 3. Run Wave Function Collapse

```rust
grid.collapse();
```

It may take few seconds...

#### 4. Spawn Sprites

```rust
grid.spawn_grid(&mut commands, &aseprite_handle);
```

See examples for more details.

# Running Example

```bash
$ cargo run --example task
```

# Credits

This project's source codes is based on the following repository:

https://github.com/webcyou-org/wave-function-collapse-rust

Article by the author(Ja):

https://qiita.com/panicdragon/items/5a02d3d1470179d77ece

See also:

https://github.com/mxgmn/WaveFunctionCollapse
