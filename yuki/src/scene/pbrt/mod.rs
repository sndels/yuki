mod cie;
mod lexer;
mod param_set;

use cie::{x_fit_1931, y_fit_1931, z_fit_1931};
use lexer::{FileLocation, Lexer, LexerError, LexerErrorType, Token};
use param_set::ParamSet;
use rayon::prelude::*;

use crate::{
    bvh::BoundingVolumeHierarchy,
    camera::FoV,
    film::FilmSettings,
    lights::{DistantLight, Light, PointLight},
    materials::{Glass, Glossy, Material, Matte, Metal},
    math::{
        transforms::{rotation, scale, translation},
        Normal, Point2, Point3, Spectrum, Transform, Vec3,
    },
    scene::{ply, CameraParameters, Scene, SceneLoadSettings},
    shapes::{Mesh, Shape, Sphere, Triangle},
    textures::ConstantTexture,
    yuki_error, yuki_info,
};

use bitflags::bitflags;
use itertools::Itertools;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Scene_Description_Interface

#[derive(Default)]
struct RenderOptions {
    camera_params: CameraParameters,
    film_settings: FilmSettings,
}

#[derive(Clone)]
struct GraphicsState {
    material: Arc<dyn Material>,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            material: get_material("matte", &ParamSet::default()),
        }
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Lexer(LexerError),
    Parser(ParserError),
    Content(String),
    Ply(String),
    Path(String),
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
    #[derive(Clone, Copy)]
    pub struct TransformBits: u8 {
        const START = 0b00001;
        const END = 0b00010;
    }
}

pub fn load(
    settings: &SceneLoadSettings,
) -> Result<(Scene, CameraParameters, FilmSettings), LoadError> {
    enum ParseShape {
        Shape(Arc<dyn Shape>),
        Mesh(Arc<Mesh>, Vec<Arc<dyn Shape>>),
        PlyMesh(PathBuf, Arc<dyn Material>, Transform<f32>),
    }

    superluminal_perf::begin_event("pbrt load");

    let mut scope_stack = vec![FileScope::new(&settings.path)?];

    let default_material = get_material("matte", &ParamSet::default());

    let mut render_options = RenderOptions::default();

    let mut graphics_state = GraphicsState::default();
    let mut current_transform: Transform<f32> = Transform::default();
    let mut active_transform_bits = TransformBits::all();

    let mut transform_stack = Vec::new();
    let mut graphics_state_stack = Vec::new();
    let mut active_transform_bits_stack = Vec::new();

    // TODO: Support instancing
    let mut parse_shapes = Vec::new();
    let mut lights: Vec<Arc<dyn Light>> = Vec::new();
    let mut background = Spectrum::zeros();
    let mut named_materials = HashMap::new();

    let parse_start = Instant::now();
    superluminal_perf::begin_event("parse");

    let mut fetched_token = None;
    let mut error_token = None;
    'scope_stack: while let Some(FileScope {
        mut lexer,
        path,
        parent_path,
    }) = scope_stack.pop()
    {
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

            macro_rules! match_bool {
                ($s:ident) => {
                    match $s.as_str() {
                        "true" => true,
                        "false" => false,
                        t => match_unexpected_token_err!(t),
                    }
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

            macro_rules! get_bool_params {
                () => {{
                    match get_next_token!() {
                        Token::String(s) => {
                            vec![match_bool!(s)]
                        }
                        Token::LeftBracket => {
                            let mut values = Vec::new();
                            loop {
                                match get_next_token!() {
                                    Token::String(s) => {
                                        values.push(match_bool!(s));
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

            macro_rules! get_two_component_vector_params {
                ($vec_type:tt, $comp_type:tt) => {{
                    match get_next_token!() {
                        Token::LeftBracket => {
                            let mut values = Vec::new();
                            loop {
                                match get_next_token!() {
                                    Token::Number(c0) => match get_next_token!() {
                                        Token::Number(c1) => {
                                            values.push($vec_type::new(
                                                c0 as $comp_type,
                                                c1 as $comp_type,
                                            ));
                                        }
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
                                "bool" => param_set.add_bool(param_name, get_bool_params!()),
                                "float" => {
                                    if (param_name == "uv") {
                                        param_set.add_uv(
                                            param_name,
                                            get_two_component_vector_params!(Point2, f32),
                                        );
                                    } else {
                                        param_set.add_f32(param_name, get_num_params!(f32));
                                    }
                                }
                                "integer" => param_set.add_i32(param_name, get_num_params!(i32)),
                                "string" => param_set.add_string(param_name, get_string_params!()),
                                "color" | "rgb" => param_set.add_spectrum(
                                    param_name,
                                    get_three_component_vector_params!(Spectrum, f32),
                                ),
                                "spectrum" => {
                                    let token = get_next_token!();
                                    let values = match token {
                                        Token::String(spd_file) => {
                                            let spd_path =
                                                parent_path.join(PathBuf::from(spd_file));
                                            let file =
                                                File::open(spd_path).map_err(LoadError::Io)?;
                                            let reader = BufReader::new(file);

                                            let mut values = Vec::new();
                                            for l in reader.lines() {
                                                if let Some(v) =
                                                    l.map_err(LoadError::Io)?.split('#').next().map(
                                                        |l_start| {
                                                            l_start
                                                                .split_whitespace()
                                                                .map(|t| t.parse::<f32>().unwrap())
                                                        },
                                                    )
                                                {
                                                    values.extend(v);
                                                }
                                            }
                                            values
                                        }
                                        _ => {
                                            fetched_token = Some(token);

                                            get_num_params!(f32)
                                        }
                                    };
                                    let (lambda, samples): (Vec<f32>, Vec<f32>) =
                                        values.chunks(2).map(|c| (c[0], c[1])).unzip();
                                    param_set.add_spectrum(
                                        param_name,
                                        vec![sampled_spectrum_into_rgb(&lambda, &samples)],
                                    )
                                }
                                "point" => param_set.add_point(
                                    param_name,
                                    get_three_component_vector_params!(Point3, f32),
                                ),
                                "normal" => param_set.add_normal(
                                    param_name,
                                    get_three_component_vector_params!(Normal, f32),
                                ),
                                "blackbody" => {
                                    yuki_info!(
                                        "'blackbody' not supported, falling back to default"
                                    );
                                    drop(get_num_params!(f32));
                                }
                                "texture" => param_set.add_string(param_name, get_string_params!()),
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
                Token::AreaLightSource => ignore_type_definition!(Token::AreaLightSource),
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
                    let res_y = params.find_i32("yresolution", 480) as u16;

                    render_options.film_settings.res.x = res_x;
                    render_options.film_settings.res.y = res_y;
                }
                Token::Integrator => ignore_type_definition!(Token::Integrator),
                Token::Include => {
                    let new_scope = FileScope::new(&parent_path.join(get_string!()))?;
                    scope_stack.push(FileScope {
                        lexer,
                        path,
                        parent_path,
                    });
                    scope_stack.push(new_scope);
                    continue 'scope_stack;
                }
                Token::LightSource => {
                    let type_name = get_string!();
                    let params = get_param_set!();
                    match type_name.as_str() {
                        "infinite" => {
                            let default_l = Spectrum::ones();
                            background = params.find_spectrum("L", default_l);
                        }
                        "distant" => {
                            let radiance = params.find_spectrum("L", Spectrum::ones());
                            if !radiance.is_black() {
                                let from = params.find_point("from", Point3::zeros());
                                let to = params.find_point("to", Point3::new(0.0, 0.0, 1.0));
                                lights.push(Arc::new(DistantLight::new(
                                    radiance,
                                    (from - to).normalized(),
                                )));
                            }
                        }
                        "point" => {
                            let default_i = Spectrum::ones();
                            let i = params.find_spectrum("I", default_i);
                            if !i.is_black() {
                                let default_pos = Point3::zeros();
                                let pos = params.find_point("from", default_pos);
                                lights.push(Arc::new(PointLight::new(&translation(pos.into()), i)));
                            }
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
                Token::NamedMaterial => {
                    let name = get_string!();
                    let material = named_materials.get(&name).unwrap_or_else(|| {
                        yuki_info!("Unknown named material '{name}'");
                        &default_material
                    });
                    graphics_state.material = Arc::clone(material);
                }
                Token::Material => {
                    graphics_state.material = get_material(&get_string!(), &get_param_set!());
                }
                Token::MakeNamedMaterial => {
                    let name = get_string!();
                    let string_type = get_string!();
                    if string_type != "string type" {
                        error_token =
                            Some((ParserErrorType::UnknownParamType, format!("{:?}", token)));
                        break 'top_parse;
                    }
                    named_materials.insert(name, get_material(&get_string!(), &get_param_set!()));
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
                    let material = Arc::clone(&graphics_state.material);
                    match shape_type.as_str() {
                        "sphere" => {
                            let radius = params.find_f32("radius", 1.0);
                            parse_shapes.push(ParseShape::Shape(Arc::new(Sphere::new(
                                &current_transform,
                                radius,
                                material,
                            ))));
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
                            let default_uvs = Vec::new();
                            let uvs = Vec::from(params.find_uvs("uv", &default_uvs));

                            let mesh = Arc::new(Mesh::new(
                                &current_transform,
                                indices,
                                points,
                                normals,
                                uvs,
                            ));
                            let tri_shapes = (0..num_indices)
                                .step_by(3)
                                .map(|v0| {
                                    Arc::new(Triangle::new(
                                        Arc::clone(&mesh),
                                        v0,
                                        Arc::clone(&material),
                                        None,
                                    )) as Arc<dyn Shape>
                                })
                                .collect();
                            parse_shapes.push(ParseShape::Mesh(mesh, tri_shapes));
                        }
                        "plymesh" => {
                            let filename = params.find_string("filename", "");
                            assert!(!filename.is_empty(), "Empty PLY filename");

                            let ply_abspath = match parent_path.join(&filename).canonicalize() {
                                Ok(p) => p,
                                Err(e) => {
                                    yuki_error!(
                                        "Error canonicalizing absolute plypath for '{}'",
                                        filename
                                    );
                                    return Err(LoadError::Io(e));
                                }
                            };

                            parse_shapes.push(ParseShape::PlyMesh(
                                ply_abspath,
                                material,
                                current_transform.clone(),
                            ));
                        }
                        t => {
                            yuki_info!("Unsupported shape type '{}'. Skipping", t);
                        }
                    }
                }
                Token::Scale => {
                    current_transform =
                        &current_transform * &scale(get_f32!(), get_f32!(), get_f32!());
                }
                Token::Texture => {
                    yuki_info!("Ignoring type definition '{:?}'", Token::Texture);
                    let _name = get_string!();
                    let _type = get_string!();
                    let _class = get_string!();
                    let _params = get_param_set!();
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
                    error_token =
                        Some((ParserErrorType::UnimplementedToken, format!("{:?}", token)));
                    break 'top_parse;
                }
            }
        }

        if let Some((error_type, token)) = error_token {
            let location = lexer.previous_token_location();
            return Err(LoadError::Parser(ParserError {
                error_type,
                token,
                file: String::from(path.to_string_lossy()),
                location,
            }));
        }
    }

    superluminal_perf::end_event(); // parse

    yuki_info!(
        "pbrt-v3: Parse took {:.2}s in total",
        parse_start.elapsed().as_secs_f32()
    );

    let ply_start = Instant::now();
    superluminal_perf::begin_event("load plys");

    parse_shapes.par_iter_mut().try_for_each(|s| match s {
        ParseShape::PlyMesh(path, material, transform) => {
            let ply::PlyResult {
                mesh,
                shapes: ply_shapes,
            } = ply::load(path, material, Some(transform.clone()))
                .map_err(|e| LoadError::Ply(e.to_string()))?;
            *s = ParseShape::Mesh(mesh, ply_shapes);
            Ok(())
        }
        _ => Ok(()),
    })?;

    superluminal_perf::end_event(); // load plys
    yuki_info!(
        "pbrt-v3: Loading PLYs took {:.2}s in total",
        ply_start.elapsed().as_secs_f32()
    );

    superluminal_perf::begin_event("collect meshes");

    let mut meshes: Vec<Arc<Mesh>> = Vec::new();
    let mut shapes: Vec<Arc<dyn Shape>> = Vec::new();

    for s in parse_shapes {
        match s {
            ParseShape::Shape(shape) => shapes.push(shape),
            ParseShape::Mesh(mesh, tri_shapes) => {
                meshes.push(mesh);
                shapes.extend(tri_shapes);
            }
            ParseShape::PlyMesh(..) => unreachable!("We should have converted these to Mesh()"),
        }
    }

    superluminal_perf::end_event(); // collect meshes

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

    let (bvh, shapes) = BoundingVolumeHierarchy::new(
        shapes,
        settings.max_shapes_in_node as usize,
        settings.split_method,
    );

    superluminal_perf::end_event(); // pbrt load

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

fn get_material(material_type: &str, params: &ParamSet) -> Arc<dyn Material> {
    match material_type {
        "glass" => {
            let kr = params.find_spectrum("Kr", Spectrum::ones());
            let kt = params.find_spectrum("Kt", Spectrum::ones());
            let eta = params.find_f32("eta", 1.5);

            Arc::new(Glass::new(
                Arc::new(ConstantTexture::new(kr)),
                Arc::new(ConstantTexture::new(kt)),
                eta,
            )) as Arc<dyn Material>
        }
        "glossy" => {
            let rs = params.find_spectrum("Rs", Spectrum::new(0.5, 0.5, 0.5));
            let roughness = params.find_f32("roughness", 0.5);
            Arc::new(Glossy::new(
                Arc::new(ConstantTexture::new(rs)),
                Arc::new(ConstantTexture::new(roughness)),
                false,
            )) as Arc<dyn Material>
        }
        "matte" => {
            let kd = params.find_spectrum("Kd", Spectrum::new(0.5, 0.5, 0.5));
            // Matte expects sigma as radians instead of degrees
            let sigma = params.find_f32("sigma", 0.0).to_radians();
            Arc::new(Matte::new(
                Arc::new(ConstantTexture::new(kd)),
                Arc::new(ConstantTexture::new(sigma.to_radians())),
            )) as Arc<dyn Material>
        }
        "metal" => {
            let eta = params.find_spectrum(
                "eta",
                sampled_spectrum_into_rgb(&COPPER_WAVELENGTHS, &COPPER_N),
            );
            let k = params.find_spectrum(
                "k",
                sampled_spectrum_into_rgb(&COPPER_WAVELENGTHS, &COPPER_K),
            );
            let roughness = params.find_f32("roughness", 0.01);
            let remap_roughness = params.find_bool("remaproughness", true);
            Arc::new(Metal::new(
                Arc::new(ConstantTexture::new(eta)),
                Arc::new(ConstantTexture::new(k)),
                Arc::new(ConstantTexture::new(roughness)),
                remap_roughness,
            )) as Arc<dyn Material>
        }
        t => {
            yuki_info!("Unsupported material type '{}'. Using default matte.", t);
            Arc::new(Matte::new(
                Arc::new(ConstantTexture::new(Spectrum::ones() * 0.5)),
                Arc::new(ConstantTexture::new(0.0)),
            )) as Arc<dyn Material>
        }
    }
}

struct FileScope {
    lexer: Lexer,
    path: PathBuf,
    parent_path: PathBuf,
}

impl FileScope {
    fn new(path: &PathBuf) -> Result<FileScope, LoadError> {
        // TODO: Very large files should be read on the fly, not as a whole
        let input: Vec<u8> = std::fs::read_to_string(path)
            .map_err(LoadError::Io)?
            .bytes()
            .collect();

        let parent_path = path
            .parent()
            .ok_or_else(|| LoadError::Path("Can't load scenes in fs root".into()))?
            .into();

        Ok(FileScope {
            lexer: Lexer::new(input),
            path: path.clone(),
            parent_path,
        })
    }
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

fn sampled_spectrum_into_rgb(lambda: &[f32], samples: &[f32]) -> Spectrum<f32> {
    assert!(
        lambda.len() == samples.len(),
        "Sample count doesn't match the number of wavelengths"
    );
    if !is_sorted(lambda) {
        let mut sorted_pairs: Vec<(&f32, &f32)> = lambda.iter().zip(samples.iter()).collect();
        sorted_pairs.sort_unstable_by(|p0, p1| p0.0.partial_cmp(p1.0).unwrap());

        let mut sorted_lambda = vec![0.0; lambda.len()];
        let mut sorted_samples = vec![0.0; samples.len()];
        for (i, p) in sorted_pairs.iter().enumerate() {
            sorted_lambda[i] = *p.0;
            sorted_samples[i] = *p.1;
        }

        sampled_spectrum_into_rgb(&sorted_lambda, &sorted_samples);
    };

    // Riemann sum
    let mut xyz = (0.0, 0.0, 0.0);
    for (&l, &s) in lambda.iter().zip(samples.iter()) {
        xyz.0 += x_fit_1931(l) * s;
        xyz.1 += y_fit_1931(l) * s;
        xyz.2 += z_fit_1931(l) * s;
    }
    let sum_scale = (lambda.last().unwrap() - lambda.first().unwrap()) / (lambda.len() as f32);
    xyz.0 *= sum_scale;
    xyz.1 *= sum_scale;
    xyz.2 *= sum_scale;

    #[allow(clippy::excessive_precision)] // In case f64 is used at some point
    Spectrum::new(
        3.240_479 * xyz.0 - 1.537_150 * xyz.1 - 0.498_535 * xyz.2,
        -0.969_256 * xyz.0 + 1.875_991 * xyz.1 + 0.041_556 * xyz.2,
        0.055_648 * xyz.0 - 0.204_043 * xyz.1 + 1.057_311 * xyz.2,
    )
}

fn is_sorted<T: PartialOrd + Copy>(values: &[T]) -> bool {
    for i in 0..(values.len() - 1) {
        if values[i].gt(&values[i + 1]) {
            return false;
        }
    }
    true
}

const N_COPPER_SAMPLES: usize = 56;
#[allow(clippy::excessive_precision, clippy::unreadable_literal)] // In case f64 is used at some point
const COPPER_WAVELENGTHS: [f32; N_COPPER_SAMPLES] = [
    298.7570554,
    302.4004341,
    306.1337728,
    309.960445,
    313.8839949,
    317.9081487,
    322.036826,
    326.2741526,
    330.6244747,
    335.092373,
    339.6826795,
    344.4004944,
    349.2512056,
    354.2405086,
    359.374429,
    364.6593471,
    370.1020239,
    375.7096303,
    381.4897785,
    387.4505563,
    393.6005651,
    399.9489613,
    406.5055016,
    413.2805933,
    420.2853492,
    427.5316483,
    435.0322035,
    442.8006357,
    450.8515564,
    459.2006593,
    467.8648226,
    476.8622231,
    486.2124627,
    495.936712,
    506.0578694,
    516.6007417,
    527.5922468,
    539.0616435,
    551.0407911,
    563.5644455,
    576.6705953,
    590.4008476,
    604.8008683,
    619.92089,
    635.8162974,
    652.5483053,
    670.1847459,
    688.8009889,
    708.4810171,
    729.3186941,
    751.4192606,
    774.9011125,
    799.8979226,
    826.5611867,
    855.0632966,
    885.6012714,
];

#[allow(clippy::excessive_precision, clippy::unreadable_literal)] // In case f64 is used at some point
const COPPER_N: [f32; N_COPPER_SAMPLES] = [
    1.400313, 1.38, 1.358438, 1.34, 1.329063, 1.325, 1.3325, 1.34, 1.334375, 1.325, 1.317812, 1.31,
    1.300313, 1.29, 1.281563, 1.27, 1.249062, 1.225, 1.2, 1.18, 1.174375, 1.175, 1.1775, 1.18,
    1.178125, 1.175, 1.172812, 1.17, 1.165312, 1.16, 1.155312, 1.15, 1.142812, 1.135, 1.131562,
    1.12, 1.092437, 1.04, 0.950375, 0.826, 0.645875, 0.468, 0.35125, 0.272, 0.230813, 0.214,
    0.20925, 0.213, 0.21625, 0.223, 0.2365, 0.25, 0.254188, 0.26, 0.28, 0.3,
];

#[allow(clippy::excessive_precision, clippy::unreadable_literal)] // In case f64 is used at some point
const COPPER_K: [f32; N_COPPER_SAMPLES] = [
    1.662125, 1.687, 1.703313, 1.72, 1.744563, 1.77, 1.791625, 1.81, 1.822125, 1.834, 1.85175,
    1.872, 1.89425, 1.916, 1.931688, 1.95, 1.972438, 2.015, 2.121562, 2.21, 2.177188, 2.13,
    2.160063, 2.21, 2.249938, 2.289, 2.326, 2.362, 2.397625, 2.433, 2.469187, 2.504, 2.535875,
    2.564, 2.589625, 2.605, 2.595562, 2.583, 2.5765, 2.599, 2.678062, 2.809, 3.01075, 3.24,
    3.458187, 3.67, 3.863125, 4.05, 4.239563, 4.43, 4.619563, 4.817, 5.034125, 5.26, 5.485625,
    5.717,
];
