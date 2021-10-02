mod lexer;
mod param_set;

use lexer::{FileLocation, Lexer, LexerError, LexerErrorType, Token};
use param_set::ParamSet;

use crate::{
    bvh::{BoundingVolumeHierarchy, SplitMethod},
    camera::FoV,
    film::FilmSettings,
    lights::{Light, PointLight},
    materials::{Glass, Material, Matte},
    math::{
        transforms::{rotation, scale, translation},
        Normal, Point3, Spectrum, Transform, Vec3,
    },
    scene::{CameraParameters, Scene, SceneLoadSettings},
    shapes::{Mesh, Shape, Sphere, Triangle},
    textures::ConstantTexture,
    yuki_error, yuki_info,
};

use bitflags::bitflags;
use itertools::Itertools;
use std::sync::Arc;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Scene_Description_Interface

#[derive(Default)]
struct RenderOptions {
    camera_params: CameraParameters,
    film_settings: FilmSettings,
}

#[derive(Clone)]
struct GraphicsState {
    material_type: String,
    material_params: ParamSet,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            material_type: String::from("matte"),
            material_params: ParamSet::default(),
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Lexer(LexerError),
    Parser(ParserError),
    Content(String),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ParserError {
    error_type: ParserErrorType,
    token: String,
    file: String,
    location: FileLocation,
}

#[derive(Debug)]
pub enum ParserErrorType {
    UnexpectedToken,
    UnimplementedToken,
    UnknownParamType,
}

bitflags! {
    pub struct TransformBits: u8 {
        const START = 0b00001;
        const END = 0b00010;
    }
}

pub fn load(
    settings: &SceneLoadSettings,
) -> Result<(Scene, CameraParameters, FilmSettings), LoadError> {
    // TODO: Very large files should be read on the fly, not as a whole
    let input: Vec<u8> = std::fs::read_to_string(&settings.path)
        .map_err(LoadError::Io)?
        .bytes()
        .collect();

    let mut lexer = Lexer::new(&input);

    let mut render_options = RenderOptions::default();

    let mut graphics_state = GraphicsState::default();
    let mut current_transform: Transform<f32> = Transform::default();
    let mut active_transform_bits = TransformBits::all();

    let mut transform_stack = Vec::new();
    let mut graphics_state_stack = Vec::new();
    let mut active_transform_bits_stack = Vec::new();

    // TODO: Support instancing
    let mut meshes = Vec::new();
    let mut shapes: Vec<Arc<dyn Shape>> = Vec::new();
    let mut lights: Vec<Arc<dyn Light>> = Vec::new();
    let mut background = Spectrum::zeros();

    let mut fetched_token = None;
    let mut error_token = None;
    'top_parse: loop {
        // TODO: How bad is this to maintain/debug in the long run?
        //       Better to just pass required context to util funcs and propagate errors with macro?
        macro_rules! get_next_token {
            () => {{
                if let Some(token) = fetched_token.take() {
                    token
                } else {
                    match lexer.next_token() {
                        Ok(v) => v,
                        Err(why) => match why {
                            LexerError {
                                error_type: LexerErrorType::EndOfInput,
                                ..
                            } => {
                                break 'top_parse;
                            }
                            err => {
                                return Err(LoadError::Lexer(err));
                            }
                        },
                    }
                }
            }};
        }

        macro_rules! match_unexpected_token_err {
            ($t:ident) => {{
                error_token = Some((ParserErrorType::UnexpectedToken, format!("{:?}", $t)));
                break 'top_parse;
            }};
        }

        macro_rules! get_num {
            ($num_type:tt) => {
                match get_next_token!() {
                    Token::Number(v) => v as $num_type,
                    t => match_unexpected_token_err!(t),
                }
            };
        }

        macro_rules! get_f32 {
            () => {
                get_num!(f32)
            };
        }

        macro_rules! try_get_string {
            () => {{
                match get_next_token!() {
                    Token::String(s) => Ok(s),
                    t => Err(t),
                }
            }};
        }

        macro_rules! get_string {
            () => {{
                match get_next_token!() {
                    Token::String(s) => s,
                    t => match_unexpected_token_err!(t),
                }
            }};
        }

        macro_rules! get_vec3 {
            () => {{
                Vec3::new(get_f32!(), get_f32!(), get_f32!())
            }};
        }

        macro_rules! get_point3 {
            () => {{
                Point3::new(get_f32!(), get_f32!(), get_f32!())
            }};
        }

        // Returns Ok((type: &str, name: &str)) or the non-matching token as Err(t: Token)
        macro_rules! get_param_def {
            () => {{
                match try_get_string!() {
                    Ok(s) => match try_split_param_def(&s) {
                        Some(v) => Ok(v),
                        None => {
                            error_token = Some((ParserErrorType::UnexpectedToken, s.into()));
                            break 'top_parse;
                        }
                    },
                    Err(err) => Err(err),
                }
            }};
        }

        macro_rules! get_num_params {
            ($num_type:tt) => {{
                match get_next_token!() {
                    Token::Number(n) => {
                        vec![n as $num_type]
                    }
                    Token::LeftBracket => {
                        let mut values = Vec::new();
                        loop {
                            match get_next_token!() {
                                Token::Number(n) => {
                                    values.push(n as $num_type);
                                }
                                Token::RightBracket => {
                                    break;
                                }
                                t => match_unexpected_token_err!(t),
                            }
                        }
                        values
                    }

                    t => match_unexpected_token_err!(t),
                }
            }};
        }

        macro_rules! get_string_params {
            () => {{
                match get_next_token!() {
                    Token::String(s) => {
                        vec![s]
                    }
                    Token::LeftBracket => {
                        let mut values = Vec::new();
                        loop {
                            match get_next_token!() {
                                Token::String(s) => {
                                    values.push(s);
                                }
                                Token::RightBracket => {
                                    break;
                                }
                                t => match_unexpected_token_err!(t),
                            }
                        }
                        values
                    }
                    t => match_unexpected_token_err!(t),
                }
            }};
        }

        macro_rules! get_three_component_vector_params {
            ($vec_type:tt, $comp_type:tt) => {{
                match get_next_token!() {
                    Token::LeftBracket => {
                        let mut values = Vec::new();
                        loop {
                            match get_next_token!() {
                                Token::Number(c0) => match get_next_token!() {
                                    Token::Number(c1) => match get_next_token!() {
                                        Token::Number(c2) => {
                                            values.push($vec_type::new(
                                                c0 as $comp_type,
                                                c1 as $comp_type,
                                                c2 as $comp_type,
                                            ));
                                        }
                                        t => match_unexpected_token_err!(t),
                                    },
                                    t => match_unexpected_token_err!(t),
                                },
                                Token::RightBracket => {
                                    break;
                                }
                                t => match_unexpected_token_err!(t),
                            }
                        }
                        values
                    }
                    t => match_unexpected_token_err!(t),
                }
            }};
        }

        macro_rules! get_param_set {
            () => {{
                let mut param_set = ParamSet::default();
                loop {
                    match get_param_def!() {
                        Ok((type_name, param_name)) => match type_name.as_str() {
                            "float" => param_set.add_f32(param_name, get_num_params!(f32)),
                            "integer" => param_set.add_i32(param_name, get_num_params!(i32)),
                            "string" => param_set.add_string(param_name, get_string_params!()),
                            "rgb" => param_set.add_spectrum(
                                param_name,
                                get_three_component_vector_params!(Spectrum, f32),
                            ),
                            "point" => param_set.add_point(
                                param_name,
                                get_three_component_vector_params!(Point3, f32),
                            ),
                            "normal" => param_set.add_normal(
                                param_name,
                                get_three_component_vector_params!(Normal, f32),
                            ),
                            "blackbody" => {
                                yuki_info!("'blackbody' not supported, falling back to default");
                                drop(get_num_params!(f32));
                            }
                            _ => {
                                error_token = Some((
                                    ParserErrorType::UnknownParamType,
                                    format!("{} {}", type_name, param_name),
                                ));
                                break 'top_parse;
                            }
                        },
                        Err(t) => {
                            fetched_token = Some(t);
                            break;
                        }
                    }
                }
                param_set
            }};
        }

        macro_rules! ignore_type_definition {
            ($token:expr) => {{
                yuki_info!("Ignoring type definition '{:?}'", $token);
                let _name = get_string!();
                let _params = get_param_set!();
            }};
        }

        let token = get_next_token!();
        match token {
            Token::ActiveTransform => {
                let token = get_next_token!();
                match token {
                    Token::All => {
                        active_transform_bits = TransformBits::all();
                    }
                    Token::StartTime => {
                        active_transform_bits = TransformBits::START;
                    }
                    Token::EndTime => {
                        active_transform_bits = TransformBits::END;
                    }
                    t => match_unexpected_token_err!(t),
                }
            }
            Token::AttributeBegin => {
                graphics_state_stack.push(graphics_state.clone());
                transform_stack.push(current_transform.clone());
                active_transform_bits_stack.push(active_transform_bits);
            }
            Token::AttributeEnd => {
                if graphics_state_stack.is_empty() {
                    yuki_error!("Unmatched 'AttributeEnd' found. Ignoring");
                } else {
                    graphics_state = graphics_state_stack.pop().unwrap();
                    current_transform = transform_stack.pop().unwrap();
                    active_transform_bits = active_transform_bits_stack.pop().unwrap();
                }
            }
            Token::Camera => {
                let name = get_string!();
                if name != "perspective" {
                    return Err(LoadError::Content(
                        "Only perspective camera is supported".into(),
                    ));
                }
                let params = get_param_set!();
                render_options.camera_params.fov = FoV::Y(params.find_f32("fov", 45.0));
            }
            Token::Film => {
                // TODO: Variants
                let _name = get_string!();
                let params = get_param_set!();

                #[allow(clippy::cast_sign_loss)] // Resolution is always positive
                let res_x = params.find_i32("xresolution", 640) as u16;
                #[allow(clippy::cast_sign_loss)] // Resolution is always positive
                let res_y = params.find_i32("yresolution", 640) as u16;

                render_options.film_settings.res.x = res_x;
                render_options.film_settings.res.y = res_y;
            }
            Token::Integrator => ignore_type_definition!(Token::Integrator),
            Token::LightSource => {
                let type_name = get_string!();
                let params = get_param_set!();
                match type_name.as_str() {
                    "infinite" => {
                        let default_l = Spectrum::ones();
                        background = params.find_spectrum("L", default_l);
                    }
                    "point" => {
                        let default_i = Spectrum::ones();
                        let i = params.find_spectrum("I", default_i);
                        let default_pos = Point3::zeros();
                        let pos = params.find_point("from", default_pos);
                        lights.push(Arc::new(PointLight::new(&translation(pos.into()), i)));
                    }
                    _ => {
                        yuki_info!("'{}' light not implemented", type_name);
                    }
                }
            }
            Token::LookAt => {
                // No support for t0, t1 yet so pick start location
                if active_transform_bits.contains(TransformBits::START) {
                    render_options.camera_params.position = get_point3!();
                    render_options.camera_params.target = get_point3!();
                    render_options.camera_params.up = get_vec3!().normalized();
                }
            }
            Token::Material => {
                graphics_state.material_type = get_string!();
                graphics_state.material_params = get_param_set!();
            }
            Token::Rotate => {
                let angle = get_f32!();
                let axis = Vec3::new(get_f32!(), get_f32!(), get_f32!());
                current_transform = &current_transform * &rotation(angle.to_radians(), axis);
            }
            Token::Sampler => ignore_type_definition!(Token::Sampler),
            Token::Shape => {
                // TODO: Transform cache? Will drop memory usage if a large number of shapes are used
                let shape_type = get_string!();
                let params = get_param_set!();
                // TODO: Named materials
                let material = match graphics_state.material_type.as_str() {
                    "matte" => {
                        let kd = graphics_state
                            .material_params
                            .find_spectrum("Kd", Spectrum::new(0.5, 0.5, 0.5));
                        // Matte expects sigma as radians instead of degrees
                        let sigma = graphics_state
                            .material_params
                            .find_f32("sigma", 0.0)
                            .to_radians();
                        Arc::new(Matte::new(
                            Arc::new(ConstantTexture::new(kd)),
                            Arc::new(ConstantTexture::new(sigma.to_radians())),
                        )) as Arc<dyn Material>
                    }
                    "glass" => {
                        let kr = graphics_state
                            .material_params
                            .find_spectrum("Kr", Spectrum::ones());
                        let kt = graphics_state
                            .material_params
                            .find_spectrum("Kt", Spectrum::ones());
                        let eta = graphics_state.material_params.find_f32("eta", 1.5);

                        Arc::new(Glass::new(
                            Arc::new(ConstantTexture::new(kr)),
                            Arc::new(ConstantTexture::new(kt)),
                            eta,
                        )) as Arc<dyn Material>
                    }
                    t => {
                        yuki_info!("Unsupported material type '{}'. Using default matte.", t);
                        Arc::new(Matte::new(
                            Arc::new(ConstantTexture::new(Spectrum::ones() * 0.5)),
                            Arc::new(ConstantTexture::new(0.0)),
                        )) as Arc<dyn Material>
                    }
                };
                match shape_type.as_str() {
                    "sphere" => {
                        let radius = params.find_f32("radius", 1.0);
                        shapes.push(Arc::new(Sphere::new(&current_transform, radius, material)));
                    }
                    "trianglemesh" => {
                        let default_indices = Vec::new();
                        #[allow(clippy::cast_sign_loss)] // Valid indices are never negative
                        let indices: Vec<usize> = params
                            .find_i32s("indices", &default_indices)
                            .iter()
                            .map(|&i| i as usize)
                            .collect();

                        let num_indices = indices.len();
                        if num_indices < 3 {
                            yuki_error!("Invalid 'trianglemesh' with less than 3 indices");
                            continue 'top_parse;
                        }
                        if num_indices % 3 != 0 {
                            yuki_error!("Invalid 'trianglemesh' with an index count that is not a multiple of 3");
                            continue 'top_parse;
                        }

                        let default_points = Vec::new();
                        let points = Vec::from(params.find_points("P", &default_points));
                        let default_normals = Vec::new();
                        let normals = Vec::from(params.find_normals("N", &default_normals));

                        let mesh =
                            Arc::new(Mesh::new(&current_transform, indices, points, normals));
                        shapes.extend((0..num_indices).step_by(3).map(|v0| {
                            Arc::new(Triangle::new(Arc::clone(&mesh), v0, Arc::clone(&material)))
                                as Arc<dyn Shape>
                        }));
                        meshes.push(mesh);
                    }
                    t => {
                        yuki_info!("Unsupported shape type '{}'. Skipping", t);
                    }
                }
            }
            Token::Scale => {
                current_transform = &current_transform * &scale(get_f32!(), get_f32!(), get_f32!());
            }
            Token::Translate => {
                let delta = Vec3::new(get_f32!(), get_f32!(), get_f32!());
                current_transform = &current_transform * &translation(delta);
            }
            Token::TransformBegin => {
                transform_stack.push(current_transform.clone());
            }
            Token::TransformEnd => {
                if graphics_state_stack.is_empty() {
                    yuki_error!("Unmatched 'AttributeEnd' found. Ignoring");
                } else {
                    graphics_state = graphics_state_stack.pop().unwrap();
                }
            }
            Token::WorldBegin => {
                current_transform = Transform::default();
            }
            Token::WorldEnd => (), // Don't enforce state rules for now
            _ => {
                error_token = Some((ParserErrorType::UnimplementedToken, format!("{:?}", token)));
                break 'top_parse;
            }
        }
    }

    // TODO: This could be much cleaner
    if render_options.film_settings.res.y < render_options.film_settings.res.x {
        render_options.camera_params.fov = FoV::Y(match render_options.camera_params.fov {
            FoV::X(angle) | FoV::Y(angle) => angle,
        });
    } else {
        render_options.camera_params.fov = FoV::X(match render_options.camera_params.fov {
            FoV::X(angle) | FoV::Y(angle) => angle,
        });
    }

    if let Some((error_type, token)) = error_token {
        let location = lexer.previous_token_location();
        return Err(LoadError::Parser(ParserError {
            error_type,
            token,
            file: String::from(settings.path.to_string_lossy()),
            location,
        }));
    }

    let (bvh, shapes) = BoundingVolumeHierarchy::new(
        shapes,
        settings.max_shapes_in_node as usize,
        SplitMethod::Middle,
    );

    Ok((
        Scene {
            name: settings.path.file_name().unwrap().to_str().unwrap().into(),
            load_settings: settings.clone(),
            meshes,
            shapes,
            bvh,
            lights,
            background,
        },
        render_options.camera_params,
        render_options.film_settings,
    ))
}

fn try_split_param_def(def: &str) -> Option<(String, String)> {
    let mut split = def.split_whitespace();
    match split.next_tuple() {
        Some((type_name, param_name)) => {
            if split.count() == 0 {
                Some((type_name.into(), param_name.into()))
            } else {
                None
            }
        }
        None => None,
    }
}
