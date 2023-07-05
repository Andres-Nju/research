//! MovieClip prototype

use crate::avm1::activation::Activation;
use crate::avm1::error::Error;
use crate::avm1::globals::matrix::gradient_object_to_matrix;
use crate::avm1::globals::{self, AVM_DEPTH_BIAS, AVM_MAX_DEPTH};
use crate::avm1::object::NativeObject;
use crate::avm1::property_decl::{define_properties_on, Declaration};
use crate::avm1::{self, Object, ScriptObject, TObject, Value};
use crate::avm_error;
use crate::avm_warn;
use crate::backend::navigator::NavigationMethod;
use crate::context::{GcContext, UpdateContext};
use crate::display_object::{
    Bitmap, DisplayObject, EditText, MovieClip, TDisplayObject, TDisplayObjectContainer,
};
use crate::ecma_conversions::f64_to_wrapping_i32;
use crate::prelude::*;
use crate::string::AvmString;
use crate::vminterface::Instantiator;
use ruffle_render::shape_utils::DrawCommand;
use swf::{
    FillStyle, Fixed8, Gradient, GradientInterpolation, GradientRecord, GradientSpread,
    LineCapStyle, LineJoinStyle, LineStyle, Rectangle, Twips,
};

macro_rules! mc_method {
    ( $fn:expr ) => {
        |activation, this, args| {
            if let Some(display_object) = this.as_display_object() {
                if let Some(movie_clip) = display_object.as_movie_clip() {
                    return $fn(movie_clip, activation, args);
                }
            }
            Ok(Value::Undefined)
        }
    };
}

macro_rules! mc_getter {
    ( $get:expr ) => {
        |activation, this, _args| {
            if let Some(display_object) = this.as_display_object() {
                if let Some(movie_clip) = display_object.as_movie_clip() {
                    return $get(movie_clip, activation);
                }
            }
            Ok(Value::Undefined)
        }
    };
}

macro_rules! mc_setter {
    ( $set:expr ) => {
        |activation, this, args| {
            if let Some(display_object) = this.as_display_object() {
                if let Some(movie_clip) = display_object.as_movie_clip() {
                    let value = args.get(0).unwrap_or(&Value::Undefined).clone();
                    $set(movie_clip, activation, value)?;
                }
            }
            Ok(Value::Undefined)
        }
    };
}

const PROTO_DECLS: &[Declaration] = declare_properties! {
    "attachMovie" => method(mc_method!(attach_movie); DONT_ENUM | DONT_DELETE);
    "createEmptyMovieClip" => method(mc_method!(create_empty_movie_clip); DONT_ENUM | DONT_DELETE | VERSION_6);
    "createTextField" => method(mc_method!(create_text_field); DONT_ENUM | DONT_DELETE);
    "duplicateMovieClip" => method(mc_method!(duplicate_movie_clip); DONT_ENUM | DONT_DELETE);
    "getBounds" => method(mc_method!(get_bounds); DONT_ENUM | DONT_DELETE);
    "getBytesLoaded" => method(mc_method!(get_bytes_loaded); DONT_ENUM | DONT_DELETE);
    "getBytesTotal" => method(mc_method!(get_bytes_total); DONT_ENUM | DONT_DELETE);
    "getDepth" => method(globals::get_depth; DONT_ENUM | DONT_DELETE | READ_ONLY | VERSION_6);
    "getInstanceAtDepth" => method(mc_method!(get_instance_at_depth); DONT_ENUM | DONT_DELETE | VERSION_7);
    "getNextHighestDepth" => method(mc_method!(get_next_highest_depth); DONT_ENUM | DONT_DELETE | VERSION_7);
    "getRect" => method(mc_method!(get_rect); DONT_ENUM | DONT_DELETE | VERSION_8);
    "getSWFVersion" => method(mc_method!(get_swf_version); DONT_ENUM | DONT_DELETE);
    "getURL" => method(mc_method!(get_url); DONT_ENUM | DONT_DELETE);
    "globalToLocal" => method(mc_method!(global_to_local); DONT_ENUM | DONT_DELETE);
    "gotoAndPlay" => method(mc_method!(goto_and_play); DONT_ENUM | DONT_DELETE);
    "gotoAndStop" => method(mc_method!(goto_and_stop); DONT_ENUM | DONT_DELETE);
    "hitTest" => method(mc_method!(hit_test); DONT_ENUM | DONT_DELETE);
    "loadMovie" => method(mc_method!(load_movie); DONT_ENUM | DONT_DELETE);
    "loadVariables" => method(mc_method!(load_variables); DONT_ENUM | DONT_DELETE);
    "localToGlobal" => method(mc_method!(local_to_global); DONT_ENUM | DONT_DELETE);
    "nextFrame" => method(mc_method!(next_frame); DONT_ENUM | DONT_DELETE);
    "play" => method(mc_method!(play); DONT_ENUM | DONT_DELETE);
    "prevFrame" => method(mc_method!(prev_frame); DONT_ENUM | DONT_DELETE);
    "setMask" => method(mc_method!(set_mask); DONT_ENUM | DONT_DELETE | VERSION_6);
    "startDrag" => method(mc_method!(start_drag); DONT_ENUM | DONT_DELETE);
    "stop" => method(mc_method!(stop); DONT_ENUM | DONT_DELETE);
    "stopDrag" => method(mc_method!(stop_drag); DONT_ENUM | DONT_DELETE);
    "swapDepths" => method(mc_method!(swap_depths); DONT_ENUM | DONT_DELETE);
    "unloadMovie" => method(mc_method!(unload_movie); DONT_ENUM | DONT_DELETE);
    "beginFill" => method(mc_method!(begin_fill); DONT_ENUM | DONT_DELETE | VERSION_6);
    "beginBitmapFill" => method(mc_method!(begin_bitmap_fill); DONT_ENUM | DONT_DELETE | VERSION_8);
    "beginGradientFill" => method(mc_method!(begin_gradient_fill); DONT_ENUM | DONT_DELETE | VERSION_6);
    "moveTo" => method(mc_method!(move_to); DONT_ENUM | DONT_DELETE | VERSION_6);
    "lineTo" => method(mc_method!(line_to); DONT_ENUM | DONT_DELETE | VERSION_6);
    "curveTo" => method(mc_method!(curve_to); DONT_ENUM | DONT_DELETE | VERSION_6);
    "endFill" => method(mc_method!(end_fill); DONT_ENUM | DONT_DELETE | VERSION_6);
    "lineStyle" => method(mc_method!(line_style); DONT_ENUM | DONT_DELETE | VERSION_6);
    "clear" => method(mc_method!(clear); DONT_ENUM | DONT_DELETE | VERSION_6);
    "attachBitmap" => method(mc_method!(attach_bitmap); DONT_ENUM | DONT_DELETE | VERSION_8);
    "removeMovieClip" => method(remove_movie_clip; DONT_ENUM | DONT_DELETE);
    "enabled" => bool(true; DONT_ENUM);
    "useHandCursor" => bool(true; DONT_ENUM);
    "transform" => property(mc_getter!(transform), mc_setter!(set_transform); DONT_ENUM | VERSION_8);
    "_lockroot" => property(mc_getter!(lock_root), mc_setter!(set_lock_root); DONT_DELETE | DONT_ENUM);
    "blendMode" => property(mc_getter!(blend_mode), mc_setter!(set_blend_mode); DONT_DELETE | DONT_ENUM);
    "scrollRect" => property(mc_getter!(scroll_rect), mc_setter!(set_scroll_rect); DONT_DELETE | DONT_ENUM | VERSION_8);
    // NOTE: `focusEnabled` is not a built-in property of MovieClip.
};

/// Implements `MovieClip`
pub fn constructor<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(this.into())
}

fn new_rectangle<'gc>(
    activation: &mut Activation<'_, 'gc>,
    rectangle: Rectangle<Twips>,
) -> Result<Value<'gc>, Error<'gc>> {
    let x = rectangle.x_min.to_pixels();
    let y = rectangle.y_min.to_pixels();
    let width = rectangle.width().to_pixels();
    let height = rectangle.height().to_pixels();
    let args = &[x.into(), y.into(), width.into(), height.into()];
    let proto = activation.context.avm1.prototypes().rectangle_constructor;
    proto.construct(activation, args)
}

fn object_to_rectangle<'gc>(
    activation: &mut Activation<'_, 'gc>,
    object: Object<'gc>,
) -> Result<Option<Rectangle<Twips>>, Error<'gc>> {
    const NAMES: &[&str] = &["x", "y", "width", "height"];
    let mut values = [0; 4];
    for (&name, value) in NAMES.iter().zip(&mut values) {
        *value = match object.get_local_stored(name, activation) {
            Some(value) => value.coerce_to_i32(activation)?,
            None => return Ok(None),
        }
    }
    let [x, y, width, height] = values;
    Ok(Some(Rectangle {
        x_min: Twips::from_pixels_i32(x),
        x_max: Twips::from_pixels_i32(x + width),
        y_min: Twips::from_pixels_i32(y),
        y_max: Twips::from_pixels_i32(y + height),
    }))
}

fn scroll_rect<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    if this.has_scroll_rect() {
        new_rectangle(activation, this.next_scroll_rect())
    } else {
        Ok(Value::Undefined)
    }
}

fn set_scroll_rect<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    value: Value<'gc>,
) -> Result<(), Error<'gc>> {
    if let Value::Object(object) = value {
        this.set_has_scroll_rect(activation.context.gc_context, true);
        if let Some(rectangle) = object_to_rectangle(activation, object)? {
            this.set_next_scroll_rect(activation.context.gc_context, rectangle);
        }
    } else {
        this.set_has_scroll_rect(activation.context.gc_context, false);
    };
    Ok(())
}

#[allow(clippy::comparison_chain)]
pub fn hit_test<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if args.len() > 1 {
        let x = args.get(0).unwrap().coerce_to_f64(activation)?;
        let y = args.get(1).unwrap().coerce_to_f64(activation)?;
        let shape = args
            .get(2)
            .map(|v| v.as_bool(activation.swf_version()))
            .unwrap_or(false);
        if x.is_finite() && y.is_finite() {
            // The docs say the point is in "Stage coordinates", but actually they are in root coordinates.
            // root can be moved via _root._x etc., so we actually have to transform from root to world space.
            let local = Point::from_pixels(x, y);
            let point = movie_clip.avm1_root().local_to_global(local);
            let ret = if shape {
                movie_clip.hit_test_shape(
                    &mut activation.context,
                    point,
                    HitTestOptions::AVM_HIT_TEST,
                )
            } else {
                movie_clip.hit_test_bounds(point)
            };
            return Ok(ret.into());
        }
    } else if args.len() == 1 {
        let other = activation.resolve_target_display_object(
            movie_clip.into(),
            *args.get(0).unwrap(),
            false,
        )?;
        if let Some(other) = other {
            return Ok(movie_clip.hit_test_object(other).into());
        }
    }

    Ok(false.into())
}

pub fn create_proto<'gc>(
    context: &mut GcContext<'_, 'gc>,
    proto: Object<'gc>,
    fn_proto: Object<'gc>,
) -> Object<'gc> {
    let object = ScriptObject::new(context.gc_context, Some(proto));
    define_properties_on(PROTO_DECLS, context, object, fn_proto);
    object.into()
}

fn attach_bitmap<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let [Value::Object(bitmap_data), ..] = args {
        if let NativeObject::BitmapData(bitmap_data) = bitmap_data.native() {
            if let Some(depth) = args.get(1) {
                let depth = depth
                    .coerce_to_i32(activation)?
                    .wrapping_add(AVM_DEPTH_BIAS);

                // TODO: Implement pixel snapping
                let _pixel_snapping = args
                    .get(2)
                    .unwrap_or(&Value::Undefined)
                    .as_bool(activation.swf_version());

                let smoothing = args
                    .get(3)
                    .unwrap_or(&Value::Undefined)
                    .as_bool(activation.swf_version());

                //TODO: do attached BitmapDatas have character ids?
                let display_object = Bitmap::new_with_bitmap_data(
                    &mut activation.context,
                    0,
                    bitmap_data,
                    smoothing,
                );
                movie_clip.replace_at_depth(&mut activation.context, display_object.into(), depth);
                display_object.post_instantiation(
                    &mut activation.context,
                    None,
                    Instantiator::Avm1,
                    true,
                );
            }
        }
    }

    Ok(Value::Undefined)
}

fn line_style<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(width) = args.get(0) {
        let width = Twips::from_pixels(width.coerce_to_f64(activation)?.clamp(0.0, 255.0));
        let color = if let Some(rgb) = args.get(1) {
            let rgb = rgb.coerce_to_u32(activation)?;
            let alpha = if let Some(alpha) = args.get(2) {
                alpha.coerce_to_f64(activation)?.clamp(0.0, 100.0)
            } else {
                100.0
            } as f32
                / 100.0
                * 255.0;
            Color::from_rgb(rgb, alpha as u8)
        } else {
            Color::BLACK
        };
        let is_pixel_hinted = args
            .get(3)
            .map_or(false, |v| v.as_bool(activation.swf_version()));
        let (allow_scale_x, allow_scale_y) = match args
            .get(4)
            .and_then(|v| v.coerce_to_string(activation).ok())
            .as_deref()
        {
            Some(v) if v == b"none" => (false, false),
            Some(v) if v == b"vertical" => (true, false),
            Some(v) if v == b"horizontal" => (false, true),
            _ => (true, true),
        };
        let cap_style = match args
            .get(5)
            .and_then(|v| v.coerce_to_string(activation).ok())
            .as_deref()
        {
            Some(v) if v == b"square" => LineCapStyle::Square,
            Some(v) if v == b"none" => LineCapStyle::None,
            _ => LineCapStyle::Round,
        };
        let join_style = match args
            .get(6)
            .and_then(|v| v.coerce_to_string(activation).ok())
            .as_deref()
        {
            Some(v) if v == b"miter" => {
                if let Some(limit) = args.get(7) {
                    let limit = limit.coerce_to_f64(activation)?.clamp(0.0, 255.0);
                    LineJoinStyle::Miter(Fixed8::from_f64(limit))
                } else {
                    LineJoinStyle::Miter(Fixed8::from_f32(3.0))
                }
            }
            Some(v) if v == b"bevel" => LineJoinStyle::Bevel,
            _ => LineJoinStyle::Round,
        };
        let line_style = LineStyle::new()
            .with_width(width)
            .with_color(color)
            .with_start_cap(cap_style)
            .with_end_cap(cap_style)
            .with_join_style(join_style)
            .with_allow_scale_x(allow_scale_x)
            .with_allow_scale_y(allow_scale_y)
            .with_is_pixel_hinted(is_pixel_hinted)
            .with_allow_close(false);
        movie_clip
            .drawing(activation.context.gc_context)
            .set_line_style(Some(line_style));
    } else {
        movie_clip
            .drawing(activation.context.gc_context)
            .set_line_style(None);
    }
    Ok(Value::Undefined)
}

fn begin_fill<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(rgb) = args.get(0) {
        let rgb = rgb.coerce_to_u32(activation)?;
        let alpha = if let Some(alpha) = args.get(1) {
            alpha.coerce_to_f64(activation)?.clamp(0.0, 100.0)
        } else {
            100.0
        } as f32
            / 100.0
            * 255.0;
        movie_clip
            .drawing(activation.context.gc_context)
            .set_fill_style(Some(FillStyle::Color(Color::from_rgb(rgb, alpha as u8))));
    } else {
        movie_clip
            .drawing(activation.context.gc_context)
            .set_fill_style(None);
    }
    Ok(Value::Undefined)
}

fn begin_bitmap_fill<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let fill_style = if let [Value::Object(bitmap_data), ..] = args {
        if let NativeObject::BitmapData(bitmap_data) = bitmap_data.native() {
            // Register the bitmap data with the drawing.
            let handle = bitmap_data
                .bitmap_handle(activation.context.gc_context, activation.context.renderer);
            let bitmap = ruffle_render::bitmap::BitmapInfo {
                handle,
                width: bitmap_data.width() as u16,
                height: bitmap_data.height() as u16,
            };
            let id = movie_clip
                .drawing(activation.context.gc_context)
                .add_bitmap(bitmap);

            let mut matrix = avm1::globals::matrix::object_to_matrix_or_default(
                args.get(1)
                    .unwrap_or(&Value::Undefined)
                    .coerce_to_object(activation),
                activation,
            )?;
            // Flash matrix is in pixels. Scale from pixels to twips.
            matrix *= Matrix::scale(Twips::TWIPS_PER_PIXEL as f32, Twips::TWIPS_PER_PIXEL as f32);

            // `repeating` defaults to true, `smoothed` to false.
            // `smoothed` parameter may not be listed in some documentation.
            let is_repeating = args
                .get(2)
                .unwrap_or(&true.into())
                .as_bool(activation.swf_version());
            let is_smoothed = args
                .get(3)
                .unwrap_or(&false.into())
                .as_bool(activation.swf_version());
            Some(FillStyle::Bitmap {
                id,
                matrix: matrix.into(),
                is_smoothed,
                is_repeating,
            })
        } else {
            None
        }
    } else {
        None
    };
    movie_clip
        .drawing(activation.context.gc_context)
        .set_fill_style(fill_style);
    Ok(Value::Undefined)
}

fn begin_gradient_fill<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let (Some(method), Some(colors), Some(alphas), Some(ratios), Some(matrix)) = (
        args.get(0),
        args.get(1),
        args.get(2),
        args.get(3),
        args.get(4),
    ) {
        let method = method.coerce_to_string(activation)?;
        let colors_object = colors.coerce_to_object(activation);
        let colors_length = colors_object.length(activation)?;
        let alphas_object = alphas.coerce_to_object(activation);
        let alphas_length = alphas_object.length(activation)?;
        let ratios_object = ratios.coerce_to_object(activation);
        let ratios_length = ratios_object.length(activation)?;
        let matrix_object = matrix.coerce_to_object(activation);
        if colors_length != alphas_length || colors_length != ratios_length {
            avm_warn!(
                activation,
                "beginGradientFill() received different sized arrays for colors, alphas and ratios"
            );
            return Ok(Value::Undefined);
        }
        let records: Result<Vec<_>, Error<'gc>> = (0..colors_length)
            .map(|i| {
                let ratio = ratios_object
                    .get_element(activation, i)
                    .coerce_to_f64(activation)?
                    .clamp(0.0, 255.0) as u8;
                let rgb = colors_object
                    .get_element(activation, i)
                    .coerce_to_u32(activation)?;
                let alpha = alphas_object
                    .get_element(activation, i)
                    .coerce_to_f64(activation)?
                    .clamp(0.0, 100.0);
                Ok(GradientRecord {
                    ratio,
                    color: Color::from_rgb(rgb, (alpha / 100.0 * 255.0) as u8),
                })
            })
            .collect();
        let records = records?;
        let matrix = gradient_object_to_matrix(matrix_object, activation)?;
        let spread = match args
            .get(5)
            .and_then(|v| v.coerce_to_string(activation).ok())
            .as_deref()
        {
            Some(v) if v == b"reflect" => GradientSpread::Reflect,
            Some(v) if v == b"repeat" => GradientSpread::Repeat,
            _ => GradientSpread::Pad,
        };
        let interpolation = match args
            .get(6)
            .and_then(|v| v.coerce_to_string(activation).ok())
            .as_deref()
        {
            Some(v) if v == b"linearRGB" => GradientInterpolation::LinearRgb,
            _ => GradientInterpolation::Rgb,
        };

        let gradient = Gradient {
            matrix: matrix.into(),
            spread,
            interpolation,
            records,
        };
        let style = if &method == b"linear" {
            FillStyle::LinearGradient(gradient)
        } else if &method == b"radial" {
            if let Some(focal_point) = args.get(7) {
                FillStyle::FocalGradient {
                    gradient,
                    focal_point: Fixed8::from_f64(focal_point.coerce_to_f64(activation)?),
                }
            } else {
                FillStyle::RadialGradient(gradient)
            }
        } else {
            avm_warn!(
                activation,
                "beginGradientFill() received invalid fill type {:?}",
                method
            );
            return Ok(Value::Undefined);
        };
        movie_clip
            .drawing(activation.context.gc_context)
            .set_fill_style(Some(style));
    } else {
        movie_clip
            .drawing(activation.context.gc_context)
            .set_fill_style(None);
    }
    Ok(Value::Undefined)
}

fn move_to<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let [x, y, ..] = args {
        let x = x.coerce_to_f64(activation)?;
        let y = y.coerce_to_f64(activation)?;
        movie_clip
            .drawing(activation.context.gc_context)
            .draw_command(DrawCommand::MoveTo(Point::from_pixels(x, y)));
    }
    Ok(Value::Undefined)
}

fn line_to<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let [x, y, ..] = args {
        let x = x.coerce_to_f64(activation)?;
        let y = y.coerce_to_f64(activation)?;
        movie_clip
            .drawing(activation.context.gc_context)
            .draw_command(DrawCommand::LineTo(Point::from_pixels(x, y)));
    }
    Ok(Value::Undefined)
}

fn curve_to<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let [control_x, control_y, anchor_x, anchor_y, ..] = args {
        let control_x = control_x.coerce_to_f64(activation)?;
        let control_y = control_y.coerce_to_f64(activation)?;
        let anchor_x = anchor_x.coerce_to_f64(activation)?;
        let anchor_y = anchor_y.coerce_to_f64(activation)?;
        movie_clip
            .drawing(activation.context.gc_context)
            .draw_command(DrawCommand::CurveTo {
                control: Point::from_pixels(control_x, control_y),
                anchor: Point::from_pixels(anchor_x, anchor_y),
            });
    }
    Ok(Value::Undefined)
}

fn end_fill<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    movie_clip
        .drawing(activation.context.gc_context)
        .set_fill_style(None);
    Ok(Value::Undefined)
}

fn clear<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    movie_clip.drawing(activation.context.gc_context).clear();
    Ok(Value::Undefined)
}

fn attach_movie<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let (export_name, new_instance_name, depth) = match &args.get(0..3) {
        Some([export_name, new_instance_name, depth]) => (
            export_name.coerce_to_string(activation)?,
            new_instance_name.coerce_to_string(activation)?,
            depth
                .coerce_to_i32(activation)?
                .wrapping_add(AVM_DEPTH_BIAS),
        ),
        _ => {
            avm_error!(activation, "MovieClip.attachMovie: Too few parameters");
            return Ok(Value::Undefined);
        }
    };
    let init_object = args.get(3);

    // TODO: What is the derivation of this max value? It shows up a few times in the AVM...
    // 2^31 - 16777220
    if depth < 0 || depth > AVM_MAX_DEPTH {
        return Ok(Value::Undefined);
    }

    if let Ok(new_clip) = activation
        .context
        .library
        .library_for_movie(movie_clip.movie())
        .ok_or("Movie is missing!")
        .and_then(|l| l.instantiate_by_export_name(export_name, activation.context.gc_context))
    {
        // Set name and attach to parent.
        new_clip.set_name(activation.context.gc_context, new_instance_name);
        movie_clip.replace_at_depth(&mut activation.context, new_clip, depth);
        let init_object = if let Some(Value::Object(init_object)) = init_object {
            Some(init_object.to_owned())
        } else {
            None
        };
        new_clip.post_instantiation(
            &mut activation.context,
            init_object,
            Instantiator::Avm1,
            true,
        );

        Ok(new_clip.object().coerce_to_object(activation).into())
    } else {
        avm_warn!(activation, "Unable to attach '{}'", export_name);
        Ok(Value::Undefined)
    }
}

fn create_empty_movie_clip<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let (new_instance_name, depth) = match &args.get(0..2) {
        Some([new_instance_name, depth]) => (
            new_instance_name.coerce_to_string(activation)?,
            depth
                .coerce_to_i32(activation)?
                .wrapping_add(AVM_DEPTH_BIAS),
        ),
        _ => {
            avm_error!(
                activation,
                "MovieClip.createEmptyMovieClip: Too few parameters"
            );
            return Ok(Value::Undefined);
        }
    };

    // Create empty movie clip.
    let swf_movie = movie_clip.movie();
    let new_clip = MovieClip::new(swf_movie, activation.context.gc_context);

    // Set name and attach to parent.
    new_clip.set_name(activation.context.gc_context, new_instance_name);
    movie_clip.replace_at_depth(&mut activation.context, new_clip.into(), depth);
    new_clip.post_instantiation(&mut activation.context, None, Instantiator::Avm1, true);

    Ok(new_clip.object())
}

fn create_text_field<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let movie = activation.base_clip().movie();
    let instance_name = args.get(0).cloned().unwrap_or(Value::Undefined);
    let depth = args
        .get(1)
        .cloned()
        .unwrap_or(Value::Undefined)
        .coerce_to_f64(activation)?;
    let x = args
        .get(2)
        .cloned()
        .unwrap_or(Value::Undefined)
        .coerce_to_f64(activation)?;
    let y = args
        .get(3)
        .cloned()
        .unwrap_or(Value::Undefined)
        .coerce_to_f64(activation)?;
    let width = args
        .get(4)
        .cloned()
        .unwrap_or(Value::Undefined)
        .coerce_to_f64(activation)?;
    let height = args
        .get(5)
        .cloned()
        .unwrap_or(Value::Undefined)
        .coerce_to_f64(activation)?;

    let text_field: DisplayObject<'gc> =
        EditText::new(&mut activation.context, movie, x, y, width, height).into();
    text_field.set_name(
        activation.context.gc_context,
        instance_name.coerce_to_string(activation)?,
    );
    movie_clip.replace_at_depth(
        &mut activation.context,
        text_field,
        (depth as Depth).wrapping_add(AVM_DEPTH_BIAS),
    );
    text_field.post_instantiation(&mut activation.context, None, Instantiator::Avm1, false);

    if activation.swf_version() >= 8 {
        //SWF8+ returns the `TextField` instance here
        Ok(text_field.object())
    } else {
        Ok(Value::Undefined)
    }
}

fn duplicate_movie_clip<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let name = match args.get(0) {
        Some(name) => name.coerce_to_string(activation)?,
        None => {
            avm_error!(
                activation,
                "MovieClip.duplicateMovieClip: Too few parameters"
            );
            return Ok(Value::Undefined);
        }
    };
    let depth = match args.get(1) {
        Some(depth) => depth.coerce_to_i32(activation)?,
        None => 0,
    };
    // Despite the docs say the `initObject` parameter is supported in Flash Player 6 and later,
    // it's not version-gated.
    let init_object = args.get(2).map(|v| v.coerce_to_object(activation));

    // `duplicateMovieClip` method uses biased depth compared to `CloneSprite`.
    let depth = depth.wrapping_add(AVM_DEPTH_BIAS);

    let new_clip = clone_sprite(
        movie_clip,
        &mut activation.context,
        name,
        depth,
        init_object,
    );

    // On SWF<6 undefined is returned.
    if activation.swf_version() < 6 {
        return Ok(Value::Undefined);
    }

    Ok(new_clip.map_or(Value::Undefined, |clip| clip.object()))
}

pub fn clone_sprite<'gc>(
    movie_clip: MovieClip<'gc>,
    context: &mut UpdateContext<'_, 'gc>,
    target: AvmString<'gc>,
    depth: Depth,
    init_object: Option<Object<'gc>>,
) -> Option<MovieClip<'gc>> {
    let Some(parent) = movie_clip.avm1_parent().and_then(|o| o.as_movie_clip()) else {
        // Can't duplicate the root!
        return None;
    };

    // TODO: What is the derivation of this max value? It shows up a few times in the AVM...
    // 2^31 - 16777220
    if depth < 0 || depth > AVM_MAX_DEPTH {
        return None;
    }

    let movie = parent.movie();
    let new_clip = if movie_clip.id() != 0 {
        // Clip from SWF; instantiate a new copy.
        let library = context.library.library_for_movie(movie).unwrap();
        library
            .instantiate_by_id(movie_clip.id(), context.gc_context)
            .unwrap()
            .as_movie_clip()
            .unwrap()
    } else {
        // Dynamically created clip; create a new empty movie clip.
        MovieClip::new(movie, context.gc_context)
    };

    // Set name and attach to parent.
    new_clip.set_name(context.gc_context, target);
    parent.replace_at_depth(context, new_clip.into(), depth);

    // Copy display properties from previous clip to new clip.
    new_clip.set_matrix(context.gc_context, *movie_clip.base().matrix());
    new_clip.set_color_transform(context.gc_context, *movie_clip.base().color_transform());

    new_clip.set_clip_event_handlers(context.gc_context, movie_clip.clip_actions().to_vec());

    *new_clip.drawing(context.gc_context) = movie_clip.drawing(context.gc_context).clone();
    // TODO: Any other properties we should copy...?
    // Definitely not ScriptObject properties.

    new_clip.post_instantiation(context, init_object, Instantiator::Avm1, true);

    Some(new_clip)
}

fn get_bytes_loaded<'gc>(
    movie_clip: MovieClip<'gc>,
    _activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(movie_clip.loaded_bytes().into())
}

fn get_bytes_total<'gc>(
    movie_clip: MovieClip<'gc>,
    _activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(movie_clip.total_bytes().into())
}

fn get_instance_at_depth<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if activation.swf_version() >= 7 {
        let depth = if let Some(depth) = args.get(0) {
            depth
                .coerce_to_i32(activation)?
                .wrapping_add(AVM_DEPTH_BIAS)
        } else {
            avm_error!(
                activation,
                "MovieClip.get_instance_at_depth: Too few parameters"
            );
            return Ok(Value::Undefined);
        };
        match movie_clip.child_by_depth(depth) {
            Some(child) => {
                // If the child doesn't have a corresponding AVM object, return mc itself.
                // NOTE: this behavior was guessed from observing behavior for Text and Graphic;
                // I didn't test other variants like Bitmap, MorphSpahe, Video
                // or objects that weren't fully initialized yet.
                match child.object() {
                    Value::Undefined => Ok(movie_clip.object()),
                    obj => Ok(obj),
                }
            }
            None => Ok(Value::Undefined),
        }
    } else {
        Ok(Value::Undefined)
    }
}

fn get_next_highest_depth<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if activation.swf_version() >= 7 {
        let depth = movie_clip
            .highest_depth()
            .wrapping_sub(AVM_DEPTH_BIAS - 1)
            .max(0);
        Ok(depth.into())
    } else {
        Ok(Value::Undefined)
    }
}

fn goto_and_play<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    goto_frame(movie_clip, activation, args, false, 0)
}

fn goto_and_stop<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    goto_frame(movie_clip, activation, args, true, 0)
}

pub fn goto_frame<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
    stop: bool,
    scene_offset: u16,
) -> Result<Value<'gc>, Error<'gc>> {
    let mut call_frame = None;

    match args.get(0).cloned().unwrap_or(Value::Undefined) {
        // A direct goto only runs if n is an integer
        Value::Number(n) if n.fract() == 0.0 => {
            // Frame #
            // Gotoing <= 0 has no effect.
            // Gotoing greater than _totalframes jumps to the last frame.
            // Wraps around as an i32.
            // TODO: -1 +1 here to match Flash's behavior.
            // We probably want to change our frame representation to 0-based.
            // Scene offset is only used by GotoFrame2 global opcode.
            call_frame = Some((movie_clip, f64_to_wrapping_i32(n)));
        }
        val => {
            // Coerce to string and search for a frame label.
            // This can direct other clips than the one this method was called on!
            let frame_path = val.coerce_to_string(activation)?;
            if let Some((clip, frame)) =
                activation.resolve_variable_path(movie_clip.into(), &frame_path)?
            {
                if let Some(clip) = clip.as_display_object().and_then(|o| o.as_movie_clip()) {
                    if let Ok(frame) = frame.parse().map(f64_to_wrapping_i32) {
                        // First try to parse as a frame number.
                        call_frame = Some((clip, frame));
                    } else if let Some(frame) =
                        clip.frame_label_to_number(frame, &activation.context)
                    {
                        // Otherwise, it's a frame label.
                        call_frame = Some((clip, frame as i32));
                    }
                }
            }
        }
    }

    if let Some((clip, frame)) = call_frame {
        let frame = frame.wrapping_sub(1);
        let frame = frame.wrapping_add(i32::from(scene_offset));
        let frame = frame.saturating_add(1);
        if frame > 0 {
            clip.goto_frame(&mut activation.context, frame as u16, stop);
        }
    }
    Ok(Value::Undefined)
}

fn next_frame<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    movie_clip.next_frame(&mut activation.context);
    Ok(Value::Undefined)
}

fn play<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    movie_clip.play(&mut activation.context);
    Ok(Value::Undefined)
}

fn prev_frame<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    movie_clip.prev_frame(&mut activation.context);
    Ok(Value::Undefined)
}

fn remove_movie_clip<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    // `removeMovieClip` can remove all types of display object,
    // e.g. `MovieClip.prototype.removeMovieClip.apply(textField);`
    if let Some(this) = this.as_display_object() {
        crate::avm1::globals::remove_display_object(this, activation);
    }

    Ok(Value::Undefined)
}

fn set_mask<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let mask = match args.get(0) {
        None => return Ok(Value::Undefined),
        Some(Value::Undefined | Value::Null) => None,
        Some(m) => {
            let start_clip = activation.target_clip_or_root();
            let mask = activation.resolve_target_display_object(start_clip, *m, false)?;
            if mask.is_none() {
                return Ok(Value::Bool(false));
            }
            mask
        }
    };
    let mc = DisplayObject::MovieClip(movie_clip);
    let context = &mut activation.context;
    mc.set_clip_depth(context.gc_context, 0);
    mc.set_masker(context.gc_context, mask, true);
    if let Some(m) = mask {
        m.set_maskee(context.gc_context, Some(mc), true);
    }
    Ok(Value::Bool(true))
}

fn start_drag<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    crate::avm1::activation::start_drag(movie_clip.into(), activation, args);
    Ok(Value::Undefined)
}

fn stop<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    movie_clip.stop(&mut activation.context);
    Ok(Value::Undefined)
}

fn stop_drag<'gc>(
    _movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    // It doesn't matter which clip we call this on; it simply stops any active drag.

    // we might not have had an opportunity to call `update_drag`
    // if AS did `startDrag(mc);stopDrag();` in one go
    // so let's do it here
    crate::player::Player::update_drag(&mut activation.context);

    *activation.context.drag_object = None;
    Ok(Value::Undefined)
}

fn swap_depths<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let arg = args.get(0).cloned().unwrap_or(Value::Undefined);

    if movie_clip.avm1_removed() {
        return Ok(Value::Undefined);
    }

    let mut parent = if let Some(parent) = movie_clip.avm1_parent().and_then(|o| o.as_movie_clip())
    {
        parent
    } else {
        return Ok(Value::Undefined);
    };

    let mut depth = None;
    if let Value::Number(n) = arg {
        depth = Some(crate::ecma_conversions::f64_to_wrapping_i32(n).wrapping_add(AVM_DEPTH_BIAS));
    } else if let Some(target) =
        activation.resolve_target_display_object(movie_clip.into(), arg, false)?
    {
        if let Some(target_parent) = target.avm1_parent() {
            if DisplayObject::ptr_eq(target_parent, parent.into()) && !target.avm1_removed() {
                depth = Some(target.depth())
            } else {
                avm_warn!(
                    activation,
                    "MovieClip.swapDepths: Objects do not have the same parent"
                );
            }
        }
    } else {
        avm_warn!(activation, "MovieClip.swapDepths: Invalid target");
    };

    if let Some(depth) = depth {
        if depth < 0 || depth > AVM_MAX_DEPTH {
            // Depth out of range; no action.
            return Ok(Value::Undefined);
        }

        if depth != movie_clip.depth() {
            parent.swap_at_depth(&mut activation.context, movie_clip.into(), depth);
            movie_clip.set_transformed_by_script(activation.context.gc_context, true);
        }
    }

    Ok(Value::Undefined)
}

fn local_to_global<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Value::Object(point) = args.get(0).unwrap_or(&Value::Undefined) {
        // localToGlobal does no coercion; it fails if the properties are not numbers.
        // It does not search the prototype chain and ignores virtual properties.
        if let (Value::Number(x), Value::Number(y)) = (
            point
                .get_local_stored("x", activation)
                .unwrap_or(Value::Undefined),
            point
                .get_local_stored("y", activation)
                .unwrap_or(Value::Undefined),
        ) {
            let local = Point::from_pixels(x, y);
            let global = movie_clip.local_to_global(local);
            point.set("x", global.x.to_pixels().into(), activation)?;
            point.set("y", global.y.to_pixels().into(), activation)?;
        } else {
            avm_warn!(
                activation,
                "MovieClip.localToGlobal: Invalid x and y properties"
            );
        }
    } else {
        avm_warn!(
            activation,
            "MovieClip.localToGlobal: Missing point parameter"
        );
    }

    Ok(Value::Undefined)
}

fn get_bounds<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let target = match args.get(0) {
        Some(val) => activation.resolve_target_display_object(movie_clip.into(), *val, false)?,
        None => Some(movie_clip.into()),
    };

    if let Some(target) = target {
        let bounds = movie_clip.bounds();
        let out_bounds = if DisplayObject::ptr_eq(movie_clip.into(), target) {
            // Getting the clips bounds in its own coordinate space; no AABB transform needed.
            bounds
        } else {
            // Transform AABB to target space.
            // Calculate the matrix to transform into the target coordinate space, and transform the above AABB.
            // Note that this doesn't produce as tight of an AABB as if we had used `bounds_with_transform` with
            // the final matrix, but this matches Flash's behavior.
            let to_global_matrix = movie_clip.local_to_global_matrix();
            let to_target_matrix = target.global_to_local_matrix().unwrap_or_default();
            to_target_matrix * to_global_matrix * bounds
        };

        let out = ScriptObject::new(
            activation.context.gc_context,
            Some(activation.context.avm1.prototypes().object),
        );
        out.set("xMin", out_bounds.x_min.to_pixels().into(), activation)?;
        out.set("yMin", out_bounds.y_min.to_pixels().into(), activation)?;
        out.set("xMax", out_bounds.x_max.to_pixels().into(), activation)?;
        out.set("yMax", out_bounds.y_max.to_pixels().into(), activation)?;
        Ok(out.into())
    } else {
        Ok(Value::Undefined)
    }
}

fn get_rect<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    // TODO: This should get the bounds ignoring strokes. Always equal to or smaller than getBounds.
    // Just defer to getBounds for now. Will have to store edge_bounds vs. shape_bounds in Graphic.
    get_bounds(movie_clip, activation, args)
}

fn get_swf_version<'gc>(
    movie_clip: MovieClip<'gc>,
    _activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let version = movie_clip.movie().version();
    Ok(if version > 0 {
        version.into()
    } else {
        (-1).into()
    })
}

pub fn get_url<'gc>(
    _movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    use crate::avm1::fscommand;

    //TODO: Error behavior if no arguments are present
    if let Some(url_val) = args.get(0) {
        let url = url_val.coerce_to_string(activation)?;
        if let Some(fscommand) = fscommand::parse(&url) {
            let fsargs_val = args.get(1).cloned().unwrap_or(Value::Undefined);
            let fsargs = fsargs_val.coerce_to_string(activation)?;
            let _ = fscommand::handle(fscommand, &fsargs, activation);
            return Ok(Value::Undefined);
        }

        let window = match args.get(1) {
            Some(window) => window.coerce_to_string(activation)?,
            None => "".into(),
        };

        let method = match args.get(2) {
            Some(Value::String(s)) => NavigationMethod::from_method_str(s),
            _ => None,
        };
        let vars_method = method.map(|m| (m, activation.locals_into_form_values()));

        activation.context.navigator.navigate_to_url(
            &url.to_utf8_lossy(),
            &window.to_utf8_lossy(),
            vars_method,
        );
    }

    Ok(Value::Undefined)
}

fn global_to_local<'gc>(
    movie_clip: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Value::Object(point) = args.get(0).unwrap_or(&Value::Undefined) {
        // globalToLocal does no coercion; it fails if the properties are not numbers.
        // It does not search the prototype chain and ignores virtual properties.
        if let (Value::Number(x), Value::Number(y)) = (
            point
                .get_local_stored("x", activation)
                .unwrap_or(Value::Undefined),
            point
                .get_local_stored("y", activation)
                .unwrap_or(Value::Undefined),
        ) {
            let global = Point::from_pixels(x, y);
            let local = movie_clip.global_to_local(global).unwrap_or(global);
            point.set("x", local.x.to_pixels().into(), activation)?;
            point.set("y", local.y.to_pixels().into(), activation)?;
        } else {
            avm_warn!(
                activation,
                "MovieClip.globalToLocal: Invalid x and y properties"
            );
        }
    } else {
        avm_warn!(
            activation,
            "MovieClip.globalToLocal: Missing point parameter"
        );
    }

    Ok(Value::Undefined)
}

fn load_movie<'gc>(
    target: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let url_val = args.get(0).cloned().unwrap_or(Value::Undefined);
    let url = url_val.coerce_to_string(activation)?;
    let method = args.get(1).cloned().unwrap_or(Value::Undefined);
    let method = NavigationMethod::from_method_str(&method.coerce_to_string(activation)?);
    let target_obj = target.object().coerce_to_object(activation);
    let request = activation.object_into_request(target_obj, url, method);
    let future = activation.context.load_manager.load_movie_into_clip(
        activation.context.player.clone(),
        DisplayObject::MovieClip(target),
        request,
        None,
        None,
        None,
    );
    activation.context.navigator.spawn_future(future);

    Ok(Value::Undefined)
}

fn load_variables<'gc>(
    target: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let url_val = args.get(0).cloned().unwrap_or(Value::Undefined);
    let url = url_val.coerce_to_string(activation)?;
    let method = args.get(1).cloned().unwrap_or(Value::Undefined);
    let method = NavigationMethod::from_method_str(&method.coerce_to_string(activation)?);
    let target = target.object().coerce_to_object(activation);
    let request = activation.object_into_request(target, url, method);
    let future = activation.context.load_manager.load_form_into_object(
        activation.context.player.clone(),
        target,
        request,
    );
    activation.context.navigator.spawn_future(future);

    Ok(Value::Undefined)
}

fn unload_movie<'gc>(
    target: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    target.avm1_unload(&mut activation.context);
    target.replace_with_movie(&mut activation.context, None, None);

    Ok(Value::Undefined)
}

fn transform<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let constructor = activation.context.avm1.prototypes().transform_constructor;
    let cloned = constructor.construct(activation, &[this.object()])?;
    Ok(cloned)
}

fn set_transform<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    value: Value<'gc>,
) -> Result<(), Error<'gc>> {
    if let Value::Object(object) = value {
        if let Some(transform) = object.as_transform_object() {
            if let Some(clip) = transform.clip() {
                let matrix = *clip.base().matrix();
                this.set_matrix(activation.context.gc_context, matrix);

                let color_transform = *clip.base().color_transform();
                this.set_color_transform(activation.context.gc_context, color_transform);

                this.set_transformed_by_script(activation.context.gc_context, true);
            }
        }
    }

    Ok(())
}

fn lock_root<'gc>(
    this: MovieClip<'gc>,
    _activation: &mut Activation<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(this.lock_root().into())
}

fn set_lock_root<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    value: Value<'gc>,
) -> Result<(), Error<'gc>> {
    let lock_root = value.as_bool(activation.swf_version());
    this.set_lock_root(activation.context.gc_context, lock_root);
    Ok(())
}

fn blend_mode<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
) -> Result<Value<'gc>, Error<'gc>> {
    let mode = AvmString::new_utf8(activation.context.gc_context, this.blend_mode().to_string());
    Ok(mode.into())
}

fn set_blend_mode<'gc>(
    this: MovieClip<'gc>,
    activation: &mut Activation<'_, 'gc>,
    value: Value<'gc>,
) -> Result<(), Error<'gc>> {
    // No-op if value is not a valid blend mode.
    if let Some(mode) = value.as_blend_mode() {
        this.set_blend_mode(activation.context.gc_context, mode);
    } else {
        tracing::error!("Unknown blend mode {value:?}");
    }
    Ok(())
}
