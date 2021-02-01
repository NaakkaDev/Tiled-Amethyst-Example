#![allow(dead_code, unused_imports)]

extern crate amethyst;
extern crate tiled;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use amethyst::{
    assets::{AssetStorage, Loader},
    core::transform::{Transform, TransformBundle},
    ecs::prelude::Entity,
    input::{get_key, is_close_requested, is_key_down, VirtualKeyCode},
    prelude::*,
    renderer::{
        Camera,
        ImageFormat,
        plugins::{RenderFlat2D, RenderToWindow},
        RenderingBundle,
        rendy::hal::command::ClearColor,
        Sprite, sprite::TextureCoordinates, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture, types::DefaultBackend,
    },
    tiles::{FlatEncoder, MortonEncoder2D, RenderTiles2D},
    utils::application_root_dir,
    window::{DisplayConfig, ScreenDimensions, Window},
};
use tiled::{LayerData, parse};


pub fn initialize_camera(world: &mut World) -> Entity {
    let (width, height) = {
        let dim = world.read_resource::<ScreenDimensions>();
        (dim.width(), dim.height())
    };
    
    let mut transform = Transform::default();
    // Camera z = 10.0 is usually a good starting point
    transform.set_translation_xyz(width * 0.5, height * 0.5, 10.0);
    
    world
        .create_entity()
        .with(Camera::standard_2d(width, height))
        .with(transform)
        .build()
}


struct GameplayState;

impl<'a, 'b> State<GameData<'a, 'b>, StateEvent> for GameplayState {
    fn on_start(&mut self, data: StateData<GameData>) {
        let world = data.world;
    
        // We need the camera to actually see anything
        initialize_camera(world);
        
        // Load the tiled map the "crude" way
        load_map(world);
    }

    fn handle_event(&mut self, _: StateData<'_, GameData<'_, '_>>, event: StateEvent) -> Trans<GameData<'a, 'b>, StateEvent> {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
                return Trans::Quit
            }
        }
        Trans::None
    }

    fn update(&mut self, data: StateData<'_, GameData<'_, '_>>) -> Trans<GameData<'a, 'b>, StateEvent> {
        data.data.update(&data.world);
        Trans::None
    }
}

fn load_map(world: &mut World) {
    // Get texture handle for the tileset image
    let texture_handle = {
        let loader = world.read_resource::<Loader>();
        let texture_storage = world.read_resource::<AssetStorage<Texture>>();
        loader.load(
            "assets/terrainTiles_default.png",
            ImageFormat::default(),
            (),
            &texture_storage
        )
    };
    
    // Load the tiled map
    let file = File::open(&Path::new("resources/assets/tiled_base64_zlib.tmx")).unwrap();
    let reader = BufReader::new(file);
    let map = parse(reader).unwrap();
    
    if let Some(map_tileset) = map.get_tileset_by_gid(1) {
        // 64 in this cases
        let tile_width = map_tileset.tile_width as i32;
        // 64 in this case
        let tile_height = map_tileset.tile_height as i32;
        // 640 in this case
        let tileset_width = &map_tileset.images[0].width;
        // 256 in this case
        let tileset_height = &map_tileset.images[0].height;
        // 4 columns
        let tileset_sprite_columns = tileset_width / tile_width as i32;
        // 2 rows
        let tileset_sprite_rows = tileset_height / tile_height as i32;
        
        // A place to store the tile sprites in
        let mut tile_sprites: Vec<Sprite> = Vec::new();
        
        // The x-axis needs to be reversed for TextureCoordinates
        for x in 0..tileset_sprite_rows {
            for y in 0..tileset_sprite_columns {
                let tileset_w = *&map_tileset.images[0].width as u32;
                let tileset_h = *&map_tileset.images[0].height as u32;
                let sprite_w = tile_width as u32;
                let sprite_h = tile_height as u32;
                let offset_x = (y * tile_width) as u32;
                let offset_y = (x * tile_height) as u32;
                let offsets = [0.0; 2];
                // Create a new `Sprite`
                let sprite = Sprite::from_pixel_values(
                    tileset_w,
                    tileset_h,
                    sprite_w,
                    sprite_h,
                    offset_x,
                    offset_y,
                    offsets,
                    false,
                    false
                );
                
                tile_sprites.push(sprite);
            }
        }
        
        // A sheet of sprites.. so all the tile sprites
        let sprite_sheet = SpriteSheet {
            texture: texture_handle,
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
        // NOTE: Only rendering the first layer
        let layer: &tiled::Layer = &map.layers[0];
        
        if let LayerData::Finite(tiles) = &layer.tiles {
        // Loop the row first and then the individual tiles on that row
        // and then switch to the next row
        // y = row number
        // x = column number
        // IMPORTANT: Bottom left is 0,0 so the tiles list needs to be reversed with .rev()
        for (y, row) in tiles.iter().rev().enumerate().clone() {
            for (x, &tile) in row.iter().enumerate() {
                // Do nothing with empty tiles
                if tile.gid == 0 {
                    continue;
                }
                
                // Tile ids start from 1 but tileset sprites start from 0
                let tile_id = tile.gid - 1;
                
                // Sprite for the tile
                let tile_sprite = SpriteRender {
                    sprite_sheet: sprite_sheet_handle.clone(),
                    sprite_number: tile_id as usize,
                };
                
                // Where we should draw the tile?
                let mut tile_transform = Transform::default();
                let x_coord = x * tile_width as usize;
                let y_coord = (y as f32 * tile_height as f32) + tile_height as f32;
                // Offset the positions by half the tile size so they're nice and snuggly on the screen
                // Alternatively could use the Sprite offsets instead: [-32.0, 32.0]. Depends on the use case I guess.
                let offset_x = tile_width as f32/2.0;
                let offset_y = -tile_height as f32/2.0;
                
                tile_transform.set_translation_xyz(
                    offset_x + x_coord as f32,
                    offset_y + y_coord as f32,
                    1.0
                );
                
                // Create the tile entity
                world
                    .create_entity()
                    .with(tile_transform)
                    .with(tile_sprite)
                    .build();
            }
            
        }
        }
    }
}

fn main() -> Result<(), amethyst::Error> {
    amethyst::Logger::from_config(Default::default())
        .level_for("gfx_backend_vulkan", amethyst::LogLevelFilter::Warn)
        .start();

    let app_root = application_root_dir()?;
    let resources = app_root.join("resources");
    let display_config_path = resources.join("display_config.ron");

    let game_data = GameDataBuilder::default()
        .with_bundle(TransformBundle::new())?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.00196, 0.23726, 0.21765, 1.0]),
                )
                .with_plugin(RenderFlat2D::default())
        )?;
    
    let mut game = Application::build(resources, GameplayState)?.build(game_data)?;
    
    game.run();
    Ok(())
}
