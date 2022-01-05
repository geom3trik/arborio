use celeste::binel::*;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Formatter};
use euclid::{Point2D, Size2D};

use crate::units::*;

#[derive(Debug)]
pub struct CelesteMap {
    pub name: String,
    pub filler: Vec<MapRectStrict>,
    pub foregrounds: Vec<CelesteMapStyleground>,
    pub backgrounds: Vec<CelesteMapStyleground>,
    pub levels: Vec<CelesteMapLevel>,
}

#[derive(Debug)]
pub struct CelesteMapLevel {
    pub name: String,
    pub bounds: MapRectStrict,
    pub color: i32,
    pub camera_offset_x: f32,
    pub camera_offset_y: f32,
    pub wind_pattern: String,
    pub space: bool,
    pub underwater: bool,
    pub whisper: bool,
    pub dark: bool,
    pub disable_down_transition: bool,

    pub music: String,
    pub alt_music: String,
    pub ambience: String,
    pub music_layers: [bool; 6],
    pub music_progress: String,
    pub ambience_progress: String,

    pub object_tiles: Vec<i32>,
    pub fg_decals: Vec<CelesteMapDecal>,
    pub bg_decals: Vec<CelesteMapDecal>,
    pub fg_tiles: Vec<char>,
    pub bg_tiles: Vec<char>,
    pub entities: Vec<CelesteMapEntity>,
    pub triggers: Vec<CelesteMapEntity>,

    pub cache: RefCell<CelesteMapLevelCache>,
}

#[derive(Default)]
pub struct CelesteMapLevelCache {
    pub render_cache_valid: bool,
    pub render_cache: Option<femtovg::ImageId>,
}

impl Debug for CelesteMapLevelCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CelesteMapLevelCache")
            .field("render_cache_valid", &self.render_cache_valid)
            .finish()
    }
}

#[derive(Debug)]
pub struct CelesteMapEntity {
    pub id: i32,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub attributes: HashMap<String, BinElAttr>,
    pub nodes: Vec<(i32, i32)>,
}

#[derive(Debug)]
pub struct CelesteMapDecal {
    pub x: i32,
    pub y: i32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub texture: String,
}

#[derive(Debug)]
pub struct CelesteMapStyleground {
    pub name: String,
    pub texture: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub loop_x: Option<bool>,
    pub loop_y: Option<bool>,
    pub scroll_x: Option<f32>,
    pub scroll_y: Option<f32>,
    pub speed_x: Option<f32>,
    pub speed_y: Option<f32>,
    pub color: Option<String>,
    pub blend_mode: Option<String>,
}


#[derive(Debug)]
pub struct CelesteMapError {
    pub kind: CelesteMapErrorType,
    pub description: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CelesteMapErrorType {
    ParseError,
    MissingChild,
    MissingAttribute,
    BadAttrType,
    OutOfRange,
}

impl fmt::Display for CelesteMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.description)
    }
}

impl Error for CelesteMapError {
    fn description(&self) -> &str {
        self.description.as_str()
    }
}

impl CelesteMapError {
    fn missing_child(parent: &str, child: &str) -> CelesteMapError {
        CelesteMapError {
            kind: CelesteMapErrorType::MissingChild,
            description: format!("Expected child of {}: {} not found", parent, child),
        }
    }

    fn missing_attribute(parent: &str, attr: &str) -> CelesteMapError {
        CelesteMapError {
            kind: CelesteMapErrorType::MissingAttribute,
            description: format!("Expected attribute of {}: {} not found", parent, attr),
        }
    }
}

impl CelesteMapLevel {
    pub fn tile(&self, pt: TilePoint, foreground: bool) -> Option<char> {
        let w = self.bounds.width() as i32 / 8;
        if pt.x < 0 || pt.x >= w {
            return None;
        }
        let tiles = if foreground {
            &self.fg_tiles
        } else {
            &self.bg_tiles
        };

        tiles.get((pt.x + pt.y * w) as usize).copied()
    }

    pub fn tile_mut(&mut self, pt: TilePoint, foreground: bool) -> Option<&mut char> {
        let w = self.bounds.width() as i32 / 8;
        if pt.x < 0 || pt.x >= w {
            return None;
        }
        let tiles = if foreground {
            &mut self.fg_tiles
        } else {
            &mut self.bg_tiles
        };

        tiles.get_mut((pt.x + pt.y * w) as usize)
    }
}

impl CelesteMap {
    pub fn level_at(&self, pt: MapPointStrict) -> Option<usize> {
        for (idx, room) in self.levels.iter().enumerate() {
            if room.bounds.contains(pt) {
                return Some(idx);
            }
        }
        None
    }
}


macro_rules! expect_elem {
    ($elem:expr, $name:expr) => {
        if ($elem.name != $name) {
            return Err(CelesteMapError {
                kind: CelesteMapErrorType::ParseError,
                description: format!("Expected {} element, found {}", $name, $elem.name)
            })
        }
    }
}

pub fn from_binfile(binfile: BinFile) -> Result<CelesteMap, CelesteMapError> {
    expect_elem!(binfile.root, "Map");

    let filler = get_child(&binfile.root, "Filler")?;
    let levels = get_child(&binfile.root, "levels")?;
    let style = get_child(&binfile.root, "Style")?;
    let style_fg = get_child(&style, "Foregrounds")?;
    let style_bg = get_child(&style, "Backgrounds")?;

    let filler_parsed = filler.children().map(|child| parse_filler_rect(child)).collect::<Result<_, CelesteMapError>>()?;
    let style_fg_parsed = style_fg.children().map(|child| parse_styleground(child)).collect::<Result<_, CelesteMapError>>()?;
    let style_bg_parsed = style_bg.children().map(|child| parse_styleground(child)).collect::<Result<_, CelesteMapError>>()?;
    let levels_parsed = levels.children().map(|child| parse_level(child)).collect::<Result<_, CelesteMapError>>()?;

    Ok(CelesteMap {
        name: binfile.package,
        filler: filler_parsed,
        foregrounds: style_fg_parsed,
        backgrounds: style_bg_parsed,
        levels: levels_parsed,
    })
}

fn parse_level(elem: &BinEl) -> Result<CelesteMapLevel, CelesteMapError> {
    expect_elem!(elem, "level");

    let x = get_attr(elem, "x")?;
    let y = get_attr(elem, "y")?;
    let width = get_attr(elem, "width")?;
    let height = get_attr(elem, "height")?;
    let object_tiles = get_optional_child(elem, "fgtiles");
    let fg_decals = get_optional_child(elem, "fgdecals");
    let bg_decals = get_optional_child(elem, "bgdecals");

    Ok(CelesteMapLevel {
        bounds: MapRectStrict {
            origin: Point2D::new(x, y),
            size: Size2D::new(width, height),
        },
        name: get_attr(elem, "name")?,
        color: get_optional_attr(elem, "c")?.unwrap_or_default(),
        camera_offset_x: get_optional_attr(elem, "cameraOffsetX")?.unwrap_or_default(),
        camera_offset_y: get_optional_attr(elem, "cameraOffsetY")?.unwrap_or_default(),
        wind_pattern: get_optional_attr(elem, "windPattern")?.unwrap_or_default(),
        space: get_optional_attr(elem, "space")?.unwrap_or_default(),
        underwater: get_optional_attr(elem, "underwater")?.unwrap_or_default(),
        whisper: get_optional_attr(elem, "whisper")?.unwrap_or_default(),
        dark: get_optional_attr(elem, "dark")?.unwrap_or_default(),
        disable_down_transition: get_optional_attr(elem, "disableDownTransition")?
            .unwrap_or_default(),

        music: get_optional_attr(elem, "music")?.unwrap_or_default(),
        alt_music: get_optional_attr(elem, "alt_music")?.unwrap_or_default(),
        ambience: get_optional_attr(elem, "ambience")?.unwrap_or_default(),
        music_layers: [
            get_optional_attr(elem, "musicLayer1")?.unwrap_or_default(),
            get_optional_attr(elem, "musicLayer2")?.unwrap_or_default(),
            get_optional_attr(elem, "musicLayer3")?.unwrap_or_default(),
            get_optional_attr(elem, "musicLayer4")?.unwrap_or_default(),
            get_optional_attr(elem, "musicLayer5")?.unwrap_or_default(),
            get_optional_attr(elem, "musicLayer6")?.unwrap_or_default(),
        ],
        music_progress: get_optional_attr(elem, "musicProgress")?.unwrap_or_default(),
        ambience_progress: get_optional_attr(elem, "ambienceProgress")?.unwrap_or_default(),

        fg_tiles: parse_fgbg_tiles(get_child(elem, "solids")?, width/8, height/8)?,
        bg_tiles: parse_fgbg_tiles(get_child(elem, "bg")?, width/8, height/8)?,
        object_tiles: match object_tiles {
            Some(v) => parse_object_tiles(v, width, height),
            None => Ok(vec![-1; (width/8 * height/8) as usize])
        }?,
        entities: get_child(elem, "entities")?.children().map(|child| parse_entity_trigger(child)).collect::<Result<_, CelesteMapError>>()?,
        triggers: get_child(elem, "triggers")?.children().map(|child| parse_entity_trigger(child)).collect::<Result<_, CelesteMapError>>()?,
        fg_decals: match fg_decals {
            Some(v) => v.children().map(|child| parse_decal(child)).collect::<Result<_, CelesteMapError>>()?,
            None => vec![],
        },
        bg_decals: match bg_decals {
            Some(v) => v.children().map(|child| parse_decal(child)).collect::<Result<_, CelesteMapError>>()?,
            None => vec![],
        },

        cache: default::Default::default(),
    })
}

fn parse_fgbg_tiles(elem: &BinEl, width: i32, height: i32) -> Result<Vec<char>, CelesteMapError> {
    let offset_x: i32 = get_optional_attr(elem, "offsetX")?.unwrap_or_default();
    let offset_y: i32 = get_optional_attr(elem, "offsetY")?.unwrap_or_default();
    let exc = Err(CelesteMapError { kind: CelesteMapErrorType::OutOfRange, description: format!("{} contains out-of-range data", elem.name)});
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }

    let mut data: Vec<char> = vec!['0'; (width * height) as usize];
    let mut x = offset_x;
    let mut y = offset_y;
    for ch in get_optional_attr::<String>(elem, "innerText")?
        .unwrap_or_default()
        .chars()
    {
        if ch == '\n' {
            x = offset_x;
            y += 1;
        } else if ch == '\r' {
        } else if x >= width || y >= height {
            // TODO remove this
            println!("{:?}", elem);
        } else {
            data[(x + y * width) as usize] = ch;
            x += 1;
        }
    }

    Ok(data)
}

fn parse_object_tiles(elem: &BinEl, width: i32, height: i32) -> Result<Vec<i32>, CelesteMapError> {
    let offset_x: i32 = get_optional_attr(elem, "offsetX")?.unwrap_or_default();
    let offset_y: i32 = get_optional_attr(elem, "offsetY")?.unwrap_or_default();
    let exc = Err(CelesteMapError { kind: CelesteMapErrorType::OutOfRange, description: format!("{} contains out-of-range data", elem.name)});
    if offset_x < 0 || offset_y < 0 {
        return exc;
    }

    let mut data: Vec<i32> = vec![-1; (width * height) as usize];
    let mut y = offset_y;

    for line in get_optional_attr::<String>(elem, "innerText")?
        .unwrap_or_default()
        .split('\n')
    {
        let mut x = offset_x;
        for num in line.split(',') {
            if num.is_empty() {
                continue;
            }
            let ch: i32 = match num.parse() {
                Err(_) => return Err(CelesteMapError { kind: CelesteMapErrorType::ParseError, description: format!("Could not parse {} as int", num)}),
                Ok(v) => v,
            };

            if x >= width || y >= height {
                return exc;
            } else {
                data[(x + y * width) as usize] = ch;
                x += 1;
            }
        }
        y += 1;
    }

    Ok(data)
}

fn parse_entity_trigger(elem: &BinEl) -> Result<CelesteMapEntity, CelesteMapError> {
    let basic_attrs: Vec<String> = vec!["id".to_string(), "x".to_string(), "y".to_string(), "width".to_string(), "height".to_string()];
    Ok(CelesteMapEntity {
        id: get_attr(elem, "id")?,
        name: elem.name.clone(),
        x: get_attr(elem, "x")?,
        y: get_attr(elem, "y")?,
        width: get_optional_attr(elem, "width")?.unwrap_or(0) as u32,
        height: get_optional_attr(elem, "height")?.unwrap_or(0) as u32,
        attributes: elem
            .attributes
            .iter()
            .map(|kv| (kv.0.clone(), kv.1.clone()))
            .filter(|kv| !basic_attrs.contains(kv.0.borrow()))
            .collect(),
        nodes: elem.children().map(|child| -> Result<(i32, i32), CelesteMapError> {
                Ok((get_attr(child, "x")?, get_attr(child, "y")?))
            }).collect::<Result<_, CelesteMapError>>()?,
    })
}

fn parse_decal(elem: &BinEl) -> Result<CelesteMapDecal, CelesteMapError> {
    Ok(CelesteMapDecal {
        x: get_attr(elem, "x")?,
        y: get_attr(elem, "y")?,
        scale_x: get_attr(elem, "scaleX")?,
        scale_y: get_attr(elem, "scaleY")?,
        texture: get_attr(elem, "texture")?,
    })
}

fn parse_filler_rect(elem: & BinEl) -> Result<MapRectStrict, CelesteMapError> {
    expect_elem!(elem, "rect");

    let x: i32 = get_attr(elem, "x")?;
    let y: i32 = get_attr(elem, "y")?;
    let w: i32 = get_attr(elem, "w")?;
    let h: i32 = get_attr(elem, "h")?;

    Ok(MapRectStrict { origin: Point2D::new(x * 8, y * 8), size: Size2D::new(w * 8, h * 8) })
}

fn parse_styleground(elem :&BinEl) -> Result<CelesteMapStyleground, CelesteMapError> {
    Ok(CelesteMapStyleground {
        name: elem.name.clone(),
        texture: get_optional_attr(elem, "texture")?,
        x: get_optional_attr(elem, "x")?,
        y: get_optional_attr(elem, "y")?,
        loop_x: get_optional_attr(elem, "loopx")?,
        loop_y: get_optional_attr(elem, "loopy")?,
        scroll_x: get_optional_attr(elem, "scrollx")?,
        scroll_y: get_optional_attr(elem, "scrolly")?,
        speed_x: get_optional_attr(elem, "speedx")?,
        speed_y: get_optional_attr(elem, "speedy")?,
        color: get_optional_attr(elem, "color")?,
        blend_mode: get_optional_attr(elem, "blendmode")?,
    })
}

fn get_optional_child<'a>(elem: &'a BinEl, name: &str) -> Option<&'a BinEl> {
    let children_of_name = elem.get(name);
    if let [ref child] = children_of_name.as_slice() {
        // if there is exactly one child
        Some(child)
    } else {
        None
    }
}

fn get_child<'a>(elem: &'a BinEl, name: &str) -> Result<&'a BinEl, CelesteMapError> {
    get_optional_child(elem, name).ok_or_else(|| CelesteMapError::missing_child(&elem.name, name))
}

fn get_optional_attr<T>(elem: &BinEl, name: &str) -> Result<Option<T>, CelesteMapError>
where
    T: AttrCoercion,
{
    if let Some(attr) = elem.attributes.get(name) {
        if let Some(coerced) = T::try_coerce(attr) {
            Ok(Some(coerced))
        } else {
            Err(CelesteMapError {
                kind: CelesteMapErrorType::BadAttrType,
                description: format!("Expected {nice}, found {:?}", attr, nice = T::NICE_NAME),
            })
        }
    } else {
        Ok(None)
    }
}

fn get_attr<T>(elem: &BinEl, name: &str) -> Result<T, CelesteMapError>
where
    T: AttrCoercion,
{
    get_optional_attr(elem, name)?
        .ok_or_else(|| CelesteMapError::missing_attribute(&elem.name, name))
}

// Trait for types that a BinElAttr can possibly be coerced to, with the logic to do the coercion
trait AttrCoercion: Sized {
    // Type name to print out when giving BadAttrType errors
    const NICE_NAME: &'static str;
    fn try_coerce(attr: &BinElAttr) -> Option<Self>;
}

impl AttrCoercion for i32 {
    const NICE_NAME: &'static str = "integer";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Int(i) => Some(i),
            BinElAttr::Float(f) => Some(f as i32),
            _ => None,
        }
    }
}
impl AttrCoercion for bool {
    const NICE_NAME: &'static str = "bool";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        if let BinElAttr::Bool(value) = *attr {
            Some(value)
        } else {
            None
        }
    }
}
impl AttrCoercion for f32 {
    const NICE_NAME: &'static str = "float";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Float(f) => Some(f),
            BinElAttr::Int(i) => Some(i as f32),
            _ => None,
        }
    }
}
impl AttrCoercion for String {
    const NICE_NAME: &'static str = "text";
    fn try_coerce(attr: &BinElAttr) -> Option<Self> {
        match *attr {
            BinElAttr::Text(ref s) => Some(s.clone()),
            BinElAttr::Int(i) => Some(i.to_string()),
            _ => None,
        }
    }
}
