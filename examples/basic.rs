use bevy::{image::ImageSamplerDescriptor, prelude::*};
use bevy_aseprite_ultra::prelude::*;
use bevy_wave_function_collapse_aseprite::Grid;

const DIMENSION: usize = 16;

const TILE_SIZE: u32 = 16;

#[derive(Resource)]
pub struct SourceImage(Handle<Aseprite>);

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

fn run_wave_function_collupse_task(
    mut commands: Commands,
    source: Res<SourceImage>,
    aseprites: Res<Assets<Aseprite>>,
    images: Res<Assets<Image>>,
) {
    if let Some(aseprite) = aseprites.get(source.0.id()) {
        if let Some(image) = images.get(aseprite.atlas_image.id()) {
            commands.remove_resource::<SourceImage>();

            let mut grid = Grid::new(&aseprite, &image, DIMENSION);

            grid.collapse();

            grid.spawn(&mut commands, &source.0);
        }
    }
}
