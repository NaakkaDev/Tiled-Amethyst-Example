#![allow(dead_code, unused_imports)]

extern crate amethyst;
extern crate tiled;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use tiled::parse;

use amethyst::core::transform::components::{Transform, GlobalTransform};
use amethyst::core::cgmath::{Vector3, Rad, Matrix4};
use amethyst::core::transform::TransformBundle;
use amethyst::assets::{AssetStorage, Loader};
use amethyst::utils::application_root_dir;
use amethyst::prelude::*;
use amethyst::ecs::prelude::{Entity};
use amethyst::input::{is_close_requested, is_key_down}; 
use amethyst::renderer::{DisplayConfig, DrawSprite, Sprite, SpriteSheet, Pipeline,
                         TextureCoordinates, Texture, RenderBundle, Stage, VirtualKeyCode,
                         PngFormat, MaterialTextureSet, Camera, Projection, ScreenDimensions, SpriteRender};


pub fn initialize_camera(world: &mut World) -> Entity {
    let (width, height) = {
        let dim = world.read_resource::<ScreenDimensions>();
        (dim.width(), dim.height())
    };

    world
        .create_entity()
        .with(Camera::from(Projection::orthographic(
            0.0, width, height, 0.0
        )))
        .with(GlobalTransform(Matrix4::from_translation(
            Vector3::new(0.0, 0.0, 1.0).into()
        )))
        .build()
}


struct GameplayState;

impl<'a, 'b> State<GameData<'a, 'b>, ()> for GameplayState {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;

        // LOAD TILESET IMAGE
        {
            let loader = world.read_resource::<Loader>();
            let texture_storage = world.read_resource::<AssetStorage<Texture>>();

            let texture_handle = loader.load(
                "assets/terrainTiles_default.png",
                PngFormat,
                Default::default(),
                (),
                &texture_storage
            );

            let mut material_texture_set = world.write_resource::<MaterialTextureSet>();
            material_texture_set.insert(9999, texture_handle);
        }
        // END

        // We need the camera to actually see anything
        initialize_camera(world);

        // Get the game window screen height
        let screen_height = {
            let dim = world.read_resource::<ScreenDimensions>();
            dim.height()
        };

        // Load the tiled map
        let file = File::open(&Path::new("assets/tiled_base64_zlib.tmx")).unwrap();
        let reader = BufReader::new(file);
        let map = parse(reader).unwrap();

        if let Some(map_tileset) = map.get_tileset_by_gid(1) {
            let tile_width = map_tileset.tile_width as i32;
            let tile_height = map_tileset.tile_height as i32;
            let tileset_width = &map_tileset.images[0].width;
            let tileset_height = &map_tileset.images[0].height;

            let tileset_sprite_columns = tileset_width / tile_width as i32;
            let tileset_sprite_offset_colums = 1.0 / tileset_sprite_columns as f32;

            let tileset_sprite_rows = tileset_height / tile_height as i32;
            let tileset_sprite_offset_rows = 1.0 / tileset_sprite_rows as f32;
            
            // A place to store the tile sprites in
            let mut tile_sprites: Vec<Sprite> = Vec::new();

            // The x-axis needs to be reversed for TextureCoordinates
            for x in (0..tileset_sprite_rows).rev() {
                for y in 0..tileset_sprite_columns {
                    
                    // Coordinates of the 64x64 tile sprite inside the whole
                    // tileset image, `terrainTiles_default.png` in this case
                    // Important: TextureCoordinates Y axis goes from BOTTOM (0.0) to TOP (1.0)
                    let tex_coords = TextureCoordinates {
                        left: y as f32 * tileset_sprite_offset_colums,
                        right: (y + 1) as f32 * tileset_sprite_offset_colums,
                        bottom: x as f32 * tileset_sprite_offset_rows,
                        top: (x + 1) as f32 * tileset_sprite_offset_rows
                    };

                    let sprite = Sprite {
                        width: tile_width as f32,
                        height: tile_height as f32,
                        offsets: [0.0, 64.0],
                        tex_coords
                    };

                    tile_sprites.push(sprite);
                }
            }

            // A sheet of sprites.. so all the tile sprites
            let sprite_sheet = SpriteSheet {
                texture_id: 9999,
                sprites: tile_sprites
            };

            // Insert the sprite sheet, which consists of all the tile sprites,
            // into world resources for later use
            let sprite_sheet_handle = {
                let loader = world.read_resource::<Loader>();
                let sprite_sheet_storage = world.read_resource::<AssetStorage<SpriteSheet>>();

                loader.load_from_data(sprite_sheet, (), &sprite_sheet_storage)
            };

            // Now that all the tile sprites/textures are loaded in
            // we can start drawing the tiles for our viewing pleasure
            let layer: &tiled::Layer = &map.layers[0];

            // Loop the row first and then the individual tiles on that row
            // and then switch to the next row
            // y = row number
            // x = column number
            for (y, row) in layer.tiles.iter().enumerate().clone() {
                for (x, &tile) in row.iter().enumerate() {
                    // Do nothing with empty tiles
                    if tile == 0 {
                        continue;
                    }

                    // Tile ids start from 1 but tileset sprites start from 0
                    let tile = tile - 1;

                    // Sprite for the tile
                    let tile_sprite = SpriteRender {
                        sprite_sheet: sprite_sheet_handle.clone(),
                        sprite_number: tile as usize,
                        flip_horizontal: false,
                        flip_vertical: false
                    };

                    // Where we should draw the tile?
                    let mut tile_transform = Transform::default();
                    let x_coord = x * tile_width as usize;
                    // Bottom Left is 0,0 so we flip it to Top Left with the
                    // ScreenDimensions.height since tiled coordinates start from top
                    let y_coord = (screen_height) - (y as f32 * tile_height as f32);

                    tile_transform.translation = Vector3::new(
                        x_coord as f32,
                        y_coord as f32,
                        -1.0
                    );

                    // Create the tile entity
                    world
                        .create_entity()
                            .with(GlobalTransform::default())
                            .with(tile_transform)
                            .with(tile_sprite)
                        .build();
                }

            }
        }
    }

    fn handle_event(&mut self, _: StateData<GameData>, event: StateEvent<()>) -> Trans<GameData<'a, 'b>, ()> {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                return Trans::Quit
            }
        }
        Trans::None
    }

    fn update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>, ()> {
        data.data.update(&data.world);
        Trans::None
    }
}

fn main() -> Result<(), amethyst::Error> {
    // amethyst::start_logger(Default::default());

    // The log level is set to error due do some spam
    amethyst::start_logger(amethyst::LoggerConfig{
        use_colors: true,
        level_filter: amethyst::LogLevelFilter::Error
    });

    let app_root = application_root_dir();
    let path = format!("{}/resources/display_config.ron", app_root);
    let config = DisplayConfig::load(&path);

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.00196, 0.23726, 0.21765, 1.0], 1.0)
            .with_pass(DrawSprite::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(TransformBundle::new())?
        .with_bundle(RenderBundle::new(pipe, Some(config)).with_sprite_sheet_processor())?;

    let mut game = Application::build("./", GameplayState)?
        .build(game_data)?;
    game.run();
    Ok(())
}
