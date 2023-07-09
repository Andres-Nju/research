//! `flash.display.BitmapData` builtin/prototype

use crate::avm2::activation::Activation;
use crate::avm2::error::argument_error;
use crate::avm2::filters::FilterAvm2Ext;
use crate::avm2::object::{BitmapDataObject, ByteArrayObject, Object, TObject, VectorObject};
use crate::avm2::value::Value;
use crate::avm2::vector::VectorStorage;
use crate::avm2::Error;
use crate::bitmap::bitmap_data::{
    BitmapData, BitmapDataWrapper, ChannelOptions, ThresholdOperation,
};
use crate::bitmap::bitmap_data::{BitmapDataDrawError, IBitmapDrawable};
use crate::bitmap::{is_size_valid, operations};
use crate::character::Character;
use crate::display_object::Bitmap;
use crate::swf::BlendMode;
use gc_arena::GcCell;
use ruffle_render::filters::Filter;
use ruffle_render::transform::Transform;
use std::str::FromStr;

pub use crate::avm2::object::bitmap_data_allocator;
use crate::avm2::parameters::{null_parameter_error, ParametersExt};
use crate::display_object::TDisplayObject;

/// Copy the static data from a given Bitmap into a new BitmapData.
///
/// `bd` is assumed to be an uninstantiated library symbol, associated with the
/// class named by `name`.
pub fn fill_bitmap_data_from_symbol<'gc>(
    activation: &mut Activation<'_, 'gc>,
    bd: Bitmap<'gc>,
) -> BitmapDataWrapper<'gc> {
    let new_bitmap_data = GcCell::allocate(activation.context.gc_context, BitmapData::default());
    new_bitmap_data
        .write(activation.context.gc_context)
        .set_pixels(
            bd.width().into(),
            bd.height().into(),
            true,
            bd.bitmap_data().read().pixels().to_vec(),
        );
    BitmapDataWrapper::new(new_bitmap_data)
}

/// Implements `flash.display.BitmapData`'s 'init' method (invoked from the AS3 constructor)
pub fn init<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        activation.super_init(this, &[])?;

        if this.as_bitmap_data().is_none() {
            let name = this.instance_of_class_definition().map(|c| c.read().name());
            let character = this
                .instance_of()
                .and_then(|t| {
                    activation
                        .context
                        .library
                        .avm2_class_registry()
                        .class_symbol(t)
                })
                .and_then(|(movie, chara_id)| {
                    activation
                        .context
                        .library
                        .library_for_movie_mut(movie)
                        .character_by_id(chara_id)
                        .cloned()
                });

            let new_bitmap_data = if let Some(Character::Bitmap(bitmap)) = character {
                // Instantiating BitmapData from an Animate-style bitmap asset
                fill_bitmap_data_from_symbol(activation, bitmap)
            } else {
                let new_bitmap_data =
                    GcCell::allocate(activation.context.gc_context, BitmapData::default());

                if character.is_some() {
                    //TODO: Determine if mismatched symbols will still work as a
                    //regular BitmapData subclass, or if this should throw
                    tracing::warn!(
                        "BitmapData subclass {:?} is associated with a non-bitmap symbol",
                        name
                    );
                }

                let width = args.get_u32(activation, 0)?;
                let height = args.get_u32(activation, 1)?;
                let transparency = args.get_bool(2);
                let fill_color = args.get_u32(activation, 3)?;

                if !is_size_valid(activation.context.swf.version(), width, height) {
                    return Err("Bitmap size is not valid".into());
                }

                new_bitmap_data
                    .write(activation.context.gc_context)
                    .init_pixels(width, height, transparency, fill_color as i32);
                BitmapDataWrapper::new(new_bitmap_data)
            };

            new_bitmap_data.init_object2(activation.context.gc_context, this);
            this.init_bitmap_data(activation.context.gc_context, new_bitmap_data);
        }
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.width`'s getter.
pub fn get_width<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        return Ok((bitmap_data.width() as i32).into());
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.height`'s getter.
pub fn get_height<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        return Ok((bitmap_data.height() as i32).into());
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.transparent`'s getter.
pub fn get_transparent<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        return Ok(bitmap_data.transparency().into());
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.scroll`.
pub fn scroll<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let x = args.get_i32(activation, 0)?;
        let y = args.get_i32(activation, 1)?;

        operations::scroll(activation.context.gc_context, bitmap_data, x, y);
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.copyPixels`.
pub fn copy_pixels<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let source_bitmap = args
            .get(0)
            .unwrap_or(&Value::Undefined)
            .coerce_to_object(activation)?;

        let source_rect = args.get_object(activation, 1, "sourceRect")?;

        let src_min_x = source_rect
            .get_public_property("x", activation)?
            .coerce_to_i32(activation)?;
        let src_min_y = source_rect
            .get_public_property("y", activation)?
            .coerce_to_i32(activation)?;
        let src_width = source_rect
            .get_public_property("width", activation)?
            .coerce_to_i32(activation)?;
        let src_height = source_rect
            .get_public_property("height", activation)?
            .coerce_to_i32(activation)?;

        let dest_point = args.get_object(activation, 2, "destPoint")?;

        let dest_x = dest_point
            .get_public_property("x", activation)?
            .coerce_to_i32(activation)?;
        let dest_y = dest_point
            .get_public_property("y", activation)?
            .coerce_to_i32(activation)?;

        if let Some(src_bitmap) = source_bitmap.as_bitmap_data() {
            src_bitmap.check_valid(activation)?;

            let mut alpha_source = None;

            if args.len() >= 4 {
                if let Some(alpha_bitmap) = args
                    .get(3)
                    .and_then(|o| o.as_object())
                    .and_then(|o| o.as_bitmap_data())
                {
                    // Testing shows that a null/undefined 'alphaPoint' parameter is treated
                    // as 'new Point(0, 0)'
                    let mut x = 0;
                    let mut y = 0;

                    if let Some(alpha_point) = args.try_get_object(activation, 4) {
                        x = alpha_point
                            .get_public_property("x", activation)?
                            .coerce_to_i32(activation)?;
                        y = alpha_point
                            .get_public_property("y", activation)?
                            .coerce_to_i32(activation)?;
                    }

                    alpha_source = Some((alpha_bitmap, (x, y)));
                }
            }

            let merge_alpha = args.get_bool(5);

            if let Some((alpha_bitmap, alpha_point)) = alpha_source {
                operations::copy_pixels_with_alpha_source(
                    activation.context.gc_context,
                    bitmap_data,
                    src_bitmap,
                    (src_min_x, src_min_y, src_width, src_height),
                    (dest_x, dest_y),
                    alpha_bitmap,
                    alpha_point,
                    merge_alpha,
                );
            } else {
                operations::copy_pixels(
                    activation.context.gc_context,
                    bitmap_data,
                    src_bitmap,
                    (src_min_x, src_min_y, src_width, src_height),
                    (dest_x, dest_y),
                    merge_alpha,
                );
            }
        }
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.getPixels`.
pub fn get_pixels<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let rectangle = args.get_object(activation, 0, "rect")?;
        let x = rectangle
            .get_public_property("x", activation)?
            .coerce_to_i32(activation)?;
        let y = rectangle
            .get_public_property("y", activation)?
            .coerce_to_i32(activation)?;
        let width = rectangle
            .get_public_property("width", activation)?
            .coerce_to_i32(activation)?;
        let height = rectangle
            .get_public_property("height", activation)?
            .coerce_to_i32(activation)?;
        let bytearray = ByteArrayObject::from_storage(
            activation,
            operations::get_pixels_as_byte_array(bitmap_data, x, y, width, height)?,
        )?;
        return Ok(bytearray.into());
    }

    Ok(Value::Undefined)
}

pub fn get_vector<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let rectangle = args.get_object(activation, 0, "rect")?;
        let x = rectangle
            .get_public_property("x", activation)?
            .coerce_to_i32(activation)?;
        let y = rectangle
            .get_public_property("y", activation)?
            .coerce_to_i32(activation)?;
        let width = rectangle
            .get_public_property("width", activation)?
            .coerce_to_i32(activation)?;
        let height = rectangle
            .get_public_property("height", activation)?
            .coerce_to_i32(activation)?;

        let pixels = operations::get_vector(bitmap_data, x, y, width, height);

        let value_type = activation.avm2().classes().uint;
        let new_storage = VectorStorage::from_values(pixels, false, value_type);

        return Ok(VectorObject::from_vector(new_storage, activation)?.into());
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.getPixel`.
pub fn get_pixel<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let x = args.get_u32(activation, 0)?;
        let y = args.get_u32(activation, 1)?;
        let col = operations::get_pixel(bitmap_data, x, y);
        return Ok(col.into());
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.getPixel32`.
pub fn get_pixel32<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let x = args.get_u32(activation, 0)?;
        let y = args.get_u32(activation, 1)?;
        let pixel = operations::get_pixel32(bitmap_data, x, y);
        return Ok((pixel as u32).into());
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.setPixel`.
pub fn set_pixel<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        let x = args.get_u32(activation, 0)?;
        let y = args.get_u32(activation, 1)?;
        let color = args.get_i32(activation, 2)?;
        operations::set_pixel(
            activation.context.gc_context,
            bitmap_data,
            x,
            y,
            color.into(),
        );
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.setPixel32`.
pub fn set_pixel32<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;

        let x = args.get_u32(activation, 0)?;
        let y = args.get_u32(activation, 1)?;
        let color = args.get_i32(activation, 2)?;

        operations::set_pixel32(activation.context.gc_context, bitmap_data, x, y, color);
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.setPixels`.
pub fn set_pixels<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let rectangle = args.get_object(activation, 0, "rect")?;

    let bytearray = args
        .get(1)
        .unwrap_or(&Value::Undefined)
        .coerce_to_object(activation)?;
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        let x = rectangle
            .get_public_property("x", activation)?
            .coerce_to_i32(activation)?;
        let y = rectangle
            .get_public_property("y", activation)?
            .coerce_to_i32(activation)?;
        let width = rectangle
            .get_public_property("width", activation)?
            .coerce_to_i32(activation)?;
        let height = rectangle
            .get_public_property("height", activation)?
            .coerce_to_i32(activation)?;

        let mut ba_write = bytearray
            .as_bytearray_mut(activation.context.gc_context)
            .ok_or("ArgumentError: Parameter must be a bytearray")?;

        operations::set_pixels_from_byte_array(
            activation.context.gc_context,
            bitmap_data,
            x,
            y,
            width,
            height,
            &mut ba_write,
        )
        .map_err(|e| e.to_avm(activation))?;
    }

    Ok(Value::Undefined)
}

/// Implements `BitmapData.copyChannel`.
pub fn copy_channel<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let source_bitmap = args
            .get(0)
            .unwrap_or(&Value::Undefined)
            .coerce_to_object(activation)?;

        let source_rect = args.get_object(activation, 1, "sourceRect")?;

        let dest_point = args.get_object(activation, 2, "destPoint")?;

        let dest_x = dest_point
            .get_public_property("x", activation)?
            .coerce_to_u32(activation)?;
        let dest_y = dest_point
            .get_public_property("y", activation)?
            .coerce_to_u32(activation)?;

        let source_channel = args.get_i32(activation, 3)?;

        let dest_channel = args.get_i32(activation, 4)?;

        if let Some(source_bitmap) = source_bitmap.as_bitmap_data() {
            //TODO: what if source is disposed
            let src_min_x = source_rect
                .get_public_property("x", activation)?
                .coerce_to_u32(activation)?;
            let src_min_y = source_rect
                .get_public_property("y", activation)?
                .coerce_to_u32(activation)?;
            let src_width = source_rect
                .get_public_property("width", activation)?
                .coerce_to_u32(activation)?;
            let src_height = source_rect
                .get_public_property("height", activation)?
                .coerce_to_u32(activation)?;

            operations::copy_channel(
                activation.context.gc_context,
                bitmap_data,
                (dest_x, dest_y),
                (src_min_x, src_min_y, src_width, src_height),
                source_bitmap,
                source_channel,
                dest_channel,
            );
        }
    }
    Ok(Value::Undefined)
}

pub fn flood_fill<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            let x = args.get_u32(activation, 0)?;
            let y = args.get_u32(activation, 1)?;
            let color = args.get_i32(activation, 2)?;

            operations::flood_fill(activation.context.gc_context, bitmap_data, x, y, color);
        }
    }

    Ok(Value::Undefined)
}

pub fn noise<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let low = args.get_u32(activation, 1)? as u8;

    let high = args.get_u32(activation, 2)? as u8;

    let channel_options = ChannelOptions::from_bits_truncate(args.get_u32(activation, 3)? as u8);

    let gray_scale = args.get_bool(4);

    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let random_seed = args.get_i32(activation, 0)?;
        operations::noise(
            activation.context.gc_context,
            bitmap_data,
            random_seed,
            low,
            high.max(low),
            channel_options,
            gray_scale,
        );
    }
    Ok(Value::Undefined)
}

pub fn color_transform<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            // TODO: Re-use `object_to_rectangle` in `movie_clip.rs`.
            let rectangle = args.get_object(activation, 0, "rect")?;
            let x = rectangle
                .get_public_property("x", activation)?
                .coerce_to_i32(activation)?;
            let y = rectangle
                .get_public_property("y", activation)?
                .coerce_to_i32(activation)?;
            let width = rectangle
                .get_public_property("width", activation)?
                .coerce_to_i32(activation)?;
            let height = rectangle
                .get_public_property("height", activation)?
                .coerce_to_i32(activation)?;

            let x_min = x.max(0) as u32;
            let x_max = (x + width) as u32;
            let y_min = y.max(0) as u32;
            let y_max = (y + height) as u32;

            let color_transform = args.get_object(activation, 1, "colorTransform")?;
            let color_transform =
                crate::avm2::globals::flash::geom::transform::object_to_color_transform(
                    color_transform,
                    activation,
                )?;

            operations::color_transform(
                activation.context.gc_context,
                bitmap_data,
                x_min,
                y_min,
                x_max,
                y_max,
                &color_transform,
            );
        }
    }

    Ok(Value::Undefined)
}

pub fn get_color_bounds_rect<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            let find_color = args.get_bool(2);

            let mask = args.get_i32(activation, 0)?;
            let color = args.get_i32(activation, 1)?;

            let (x, y, w, h) = operations::color_bounds_rect(bitmap_data, find_color, mask, color);

            let rect = activation
                .avm2()
                .classes()
                .rectangle
                .construct(activation, &[x.into(), y.into(), w.into(), h.into()])?
                .into();
            return Ok(rect);
        }
    }

    Ok(Value::Undefined)
}

pub fn lock<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    // `BitmapData.lock` tells Flash Player to temporarily stop updating the player's
    // dirty region for any Bitmap stage instances displaying this BitmapData.
    // Normally, each call to `setPixel` etc. causes Flash to update the player dirty
    // region with the changed area.
    //
    // Note that `lock` has no effect on future `BitmapData` operations, they will always
    // see the latest pixel data. Instead, it potentially delays the re-rendering of `Bitmap`
    // instances on the stage, based on how the player decides to update its dirty region
    // ("Show Redraw Regions" in Flash Player debugger context menu).
    //
    // Ruffle has no concept of a player dirty region for now, so this has no effect.
    Ok(Value::Undefined)
}

pub fn unlock<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    // No effect (see comments for `lock`).
    Ok(Value::Undefined)
}

pub fn hit_test<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|t| t.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            let first_point = args.get_object(activation, 0, "firstPoint")?;
            let top_left = (
                first_point
                    .get_public_property("x", activation)?
                    .coerce_to_i32(activation)?,
                first_point
                    .get_public_property("y", activation)?
                    .coerce_to_i32(activation)?,
            );
            let source_threshold = args.get_u32(activation, 1)?;
            let compare_object = args.get_object(activation, 2, "secondObject")?;
            let point_class = activation.avm2().classes().point;
            let rectangle_class = activation.avm2().classes().rectangle;

            if compare_object.is_of_type(point_class, activation) {
                let test_point = (
                    compare_object
                        .get_public_property("x", activation)?
                        .coerce_to_i32(activation)?
                        - top_left.0,
                    compare_object
                        .get_public_property("y", activation)?
                        .coerce_to_i32(activation)?
                        - top_left.1,
                );
                return Ok(Value::Bool(operations::hit_test_point(
                    bitmap_data,
                    source_threshold,
                    test_point,
                )));
            } else if compare_object.is_of_type(rectangle_class, activation) {
                let test_point = (
                    compare_object
                        .get_public_property("x", activation)?
                        .coerce_to_i32(activation)?
                        - top_left.0,
                    compare_object
                        .get_public_property("y", activation)?
                        .coerce_to_i32(activation)?
                        - top_left.1,
                );
                let size = (
                    compare_object
                        .get_public_property("width", activation)?
                        .coerce_to_i32(activation)?,
                    compare_object
                        .get_public_property("height", activation)?
                        .coerce_to_i32(activation)?,
                );
                return Ok(Value::Bool(operations::hit_test_rectangle(
                    bitmap_data,
                    source_threshold,
                    test_point,
                    size,
                )));
            } else if let Some(other_bmd) = compare_object.as_bitmap_data() {
                other_bmd.check_valid(activation)?;
                let second_point = args.get_object(activation, 3, "secondBitmapDataPoint")?;
                let second_point = (
                    second_point
                        .get_public_property("x", activation)?
                        .coerce_to_i32(activation)?,
                    second_point
                        .get_public_property("y", activation)?
                        .coerce_to_i32(activation)?,
                );
                let second_threshold = args.get_u32(activation, 4)?;

                let result = operations::hit_test_bitmapdata(
                    bitmap_data,
                    top_left,
                    source_threshold,
                    other_bmd,
                    second_point,
                    second_threshold,
                );
                return Ok(Value::Bool(result));
            } else if let Some(bitmap) = compare_object
                .as_display_object()
                .and_then(|dobj| dobj.as_bitmap())
            {
                let other_bmd = bitmap.bitmap_data_wrapper();
                other_bmd.check_valid(activation)?;
                let second_point = args.get_object(activation, 3, "secondBitmapDataPoint")?;
                let second_point = (
                    second_point
                        .get_public_property("x", activation)?
                        .coerce_to_i32(activation)?,
                    second_point
                        .get_public_property("y", activation)?
                        .coerce_to_i32(activation)?,
                );
                let second_threshold = args.get_u32(activation, 4)?;

                return Ok(Value::Bool(operations::hit_test_bitmapdata(
                    bitmap_data,
                    top_left,
                    source_threshold,
                    other_bmd,
                    second_point,
                    second_threshold,
                )));
            } else {
                // This is the error message Flash Player produces. Even though it's misleading.
                return Err(Error::AvmError(argument_error(
                    activation,
                    "Parameter 0 is of the incorrect type. Should be type BitmapData.",
                    2005,
                )?));
            }
        }
    }

    Ok(false.into())
}

/// Implements `BitmapData.draw`
pub fn draw<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        let mut transform = Transform::default();
        let mut blend_mode = BlendMode::Normal;

        if let Some(matrix) = args.try_get_object(activation, 1) {
            transform.matrix =
                crate::avm2::globals::flash::geom::transform::object_to_matrix(matrix, activation)?;
        }

        if let Some(color_transform) = args.try_get_object(activation, 2) {
            transform.color_transform =
                crate::avm2::globals::flash::geom::transform::object_to_color_transform(
                    color_transform,
                    activation,
                )?;
        }

        if let Some(mode) = args.try_get_string(activation, 3)? {
            if let Ok(mode) = BlendMode::from_str(&mode.to_string()) {
                blend_mode = mode;
            } else {
                tracing::error!("Unknown blend mode {:?}", mode);
                return Err("ArgumentError: Error #2008: Parameter blendMode must be one of the accepted values.".into());
            }
        }

        let mut clip_rect = None;

        if let Some(clip_rect_obj) = args.try_get_object(activation, 4) {
            clip_rect = Some(super::display_object::object_to_rectangle(
                activation,
                clip_rect_obj,
            )?);
        }

        let smoothing = args.get_bool(5);

        let source = args.get_object(activation, 0, "source")?;

        let source = if let Some(source_object) = source.as_display_object() {
            IBitmapDrawable::DisplayObject(source_object)
        } else if let Some(source_bitmap) = source.as_bitmap_data() {
            IBitmapDrawable::BitmapData(source_bitmap)
        } else {
            return Err(format!("BitmapData.draw: unexpected source {source:?}").into());
        };

        // If the bitmapdata is invalid, it's fine to return early, since the pixels
        // are inaccessible
        bitmap_data.check_valid(activation)?;

        // Do this last, so that we only call `overwrite_cpu_pixels_from_gpu`
        // if we're actually going to draw something.
        let quality = activation.context.stage.quality();
        match operations::draw(
            &mut activation.context,
            bitmap_data,
            source,
            transform,
            smoothing,
            blend_mode,
            clip_rect,
            quality,
        ) {
            Ok(()) => {}
            Err(BitmapDataDrawError::Unimplemented) => {
                return Err("Render backend does not support BitmapData.draw".into());
            }
        };
    }
    Ok(Value::Undefined)
}

/// Implements `BitmapData.drawWithQuality`
pub fn draw_with_quality<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        let mut transform = Transform::default();
        let mut blend_mode = BlendMode::Normal;

        if let Some(matrix) = args.try_get_object(activation, 1) {
            transform.matrix =
                crate::avm2::globals::flash::geom::transform::object_to_matrix(matrix, activation)?;
        }

        if let Some(color_transform) = args.try_get_object(activation, 2) {
            transform.color_transform =
                crate::avm2::globals::flash::geom::transform::object_to_color_transform(
                    color_transform,
                    activation,
                )?;
        }

        if let Some(mode) = args.try_get_string(activation, 3)? {
            if let Ok(mode) = BlendMode::from_str(&mode.to_string()) {
                blend_mode = mode;
            } else {
                tracing::error!("Unknown blend mode {:?}", mode);
                return Err("ArgumentError: Error #2008: Parameter blendMode must be one of the accepted values.".into());
            }
        }

        let mut clip_rect = None;

        if let Some(clip_rect_obj) = args.try_get_object(activation, 4) {
            clip_rect = Some(super::display_object::object_to_rectangle(
                activation,
                clip_rect_obj,
            )?);
        }

        let smoothing = args.get_bool(5);

        let source = args.get_object(activation, 0, "source")?;

        let source = if let Some(source_object) = source.as_display_object() {
            IBitmapDrawable::DisplayObject(source_object)
        } else if let Some(source_bitmap) = source.as_bitmap_data() {
            IBitmapDrawable::BitmapData(source_bitmap)
        } else {
            return Err(format!("BitmapData.drawWithQuality: unexpected source {source:?}").into());
        };

        // Unknown quality defaults to stage's quality
        let quality = if let Some(quality) = args.try_get_string(activation, 6)? {
            match quality.parse() {
                Ok(quality) => quality,
                Err(_) => {
                    return Err(Error::AvmError(argument_error(
                        activation,
                        "One of the parameters is invalid.",
                        2004,
                    )?));
                }
            }
        } else {
            activation.context.stage.quality()
        };

        match operations::draw(
            &mut activation.context,
            bitmap_data,
            source,
            transform,
            smoothing,
            blend_mode,
            clip_rect,
            quality,
        ) {
            Ok(()) => {}
            Err(BitmapDataDrawError::Unimplemented) => {
                return Err("Render backend does not support BitmapData.draw".into());
            }
        };
    }
    Ok(Value::Undefined)
}

/// Implement `BitmapData.fillRect`
pub fn fill_rect<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let rectangle = args.get_object(activation, 0, "rect")?;

    let color = args.get_i32(activation, 1)?;

    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let x = rectangle
            .get_public_property("x", activation)?
            .coerce_to_i32(activation)?;
        let y = rectangle
            .get_public_property("y", activation)?
            .coerce_to_i32(activation)?;
        let width = rectangle
            .get_public_property("width", activation)?
            .coerce_to_i32(activation)?;
        let height = rectangle
            .get_public_property("height", activation)?
            .coerce_to_i32(activation)?;

        operations::fill_rect(
            activation.context.gc_context,
            bitmap_data,
            x,
            y,
            width,
            height,
            color,
        );
    }
    Ok(Value::Undefined)
}

/// Implements `BitmapData.dispose`
pub fn dispose<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        // Don't check if we've already disposed this BitmapData - 'BitmapData.dispose()' can be called
        // multiple times
        bitmap_data.dispose(activation.context.gc_context);
    }
    Ok(Value::Undefined)
}

/// Implement `BitmapData.rect`
pub fn get_rect<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        return Ok(activation
            .avm2()
            .classes()
            .rectangle
            .construct(
                activation,
                &[
                    0.into(),
                    0.into(),
                    bitmap_data.width().into(),
                    bitmap_data.height().into(),
                ],
            )?
            .into());
    }
    Ok(Value::Undefined)
}

/// Implement `BitmapData.applyFilter`
pub fn apply_filter<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(dest_bitmap) = this.and_then(|this| this.as_bitmap_data()) {
        let source_bitmap = args.get_object(activation, 0, "sourceBitmapData")?
            .as_bitmap_data()
            .ok_or_else(|| {
                Error::from(format!("TypeError: Error #1034: Type Coercion failed: cannot convert {} to flash.display.BitmapData.", args[0].coerce_to_string(activation).unwrap_or_default()))
            })?;
        let source_rect = args.get_object(activation, 1, "sourceRect")?;
        let source_rect = super::display_object::object_to_rectangle(activation, source_rect)?;
        let source_point = (
            source_rect.x_min.to_pixels().floor() as u32,
            source_rect.y_min.to_pixels().floor() as u32,
        );
        let source_size = (
            source_rect.width().to_pixels().ceil() as u32,
            source_rect.height().to_pixels().ceil() as u32,
        );
        let dest_point = args.get_object(activation, 2, "dstPoint")?;
        let dest_point = (
            dest_point
                .get_public_property("x", activation)?
                .coerce_to_u32(activation)?,
            dest_point
                .get_public_property("y", activation)?
                .coerce_to_u32(activation)?,
        );
        let filter = args.get_object(activation, 3, "filter")?;
        let filter = Filter::from_avm2_object(activation, filter)?;
        operations::apply_filter(
            &mut activation.context,
            dest_bitmap,
            source_bitmap,
            source_point,
            source_size,
            dest_point,
            filter,
        )
    }
    Ok(Value::Undefined)
}

/// Implement `BitmapData.clone`
pub fn clone<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            let new_bitmap_data = operations::clone(bitmap_data);

            let class = activation.avm2().classes().bitmapdata;
            let new_bitmap_data_object = BitmapDataObject::from_bitmap_data(
                activation,
                BitmapDataWrapper::new(GcCell::allocate(
                    activation.context.gc_context,
                    new_bitmap_data,
                )),
                class,
            )?;

            return Ok(new_bitmap_data_object.into());
        }
    }
    Ok(Value::Undefined)
}

/// Implement `BitmapData.paletteMap`
pub fn palette_map<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        bitmap_data.check_valid(activation)?;
        let source_bitmap = args
            .get_object(activation, 0, "sourceBitmapData")?
            .as_bitmap_data()
            .unwrap();

        let source_rect = args.get_object(activation, 1, "sourceRect")?;
        let source_rect = super::display_object::object_to_rectangle(activation, source_rect)?;
        let source_point = (
            source_rect.x_min.to_pixels().floor() as i32,
            source_rect.y_min.to_pixels().floor() as i32,
        );
        let source_size = (
            source_rect.width().to_pixels().ceil() as i32,
            source_rect.height().to_pixels().ceil() as i32,
        );
        let dest_point = args.get_object(activation, 2, "dstPoint")?;
        let dest_point = (
            dest_point
                .get_public_property("x", activation)?
                .coerce_to_i32(activation)?,
            dest_point
                .get_public_property("x", activation)?
                .coerce_to_i32(activation)?,
        );

        let mut get_channel = |index: usize, shift: usize| -> Result<[u32; 256], Error<'gc>> {
            let arg = args.get(index).unwrap_or(&Value::Null);
            let mut array = [0_u32; 256];
            for (i, item) in array.iter_mut().enumerate() {
                *item = if let Value::Object(arg) = arg {
                    arg.get_enumerant_value(i as u32, activation)?
                        .coerce_to_u32(activation)?
                } else {
                    // This is an "identity mapping", fulfilling the part of the spec that
                    // says that channels which have no array provided are simply copied.
                    (i << shift) as u32
                }
            }
            Ok(array)
        };

        let red_array = get_channel(3, 16)?;
        let green_array = get_channel(4, 8)?;
        let blue_array = get_channel(5, 0)?;
        let alpha_array = get_channel(6, 24)?;

        operations::palette_map(
            activation.context.gc_context,
            bitmap_data,
            source_bitmap,
            (source_point.0, source_point.1, source_size.0, source_size.1),
            dest_point,
            (red_array, green_array, blue_array, alpha_array),
        );
    }

    Ok(Value::Undefined)
}

/// Implement `BitmapData.perlinNoise`
pub fn perlin_noise<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            let base_x = args.get_f64(activation, 0)?;
            let base_y = args.get_f64(activation, 1)?;
            let num_octaves = args.get_u32(activation, 2)? as usize;
            let seed = args.get_i32(activation, 3)? as i64;
            let stitch = args.get_bool(4);
            let fractal_noise = args.get_bool(5);
            let channel_options =
                ChannelOptions::from_bits_truncate(args.get_i32(activation, 6)? as u8);
            let grayscale = args.get_bool(7);
            let offsets = args.try_get_object(activation, 8);

            let octave_offsets: Result<Vec<_>, Error<'gc>> = (0..num_octaves)
                .map(|i| {
                    if let Some(offsets) = offsets {
                        if let Some(offsets) = offsets.as_array_storage() {
                            if let Some(Value::Object(e)) = offsets.get(i) {
                                let x = e
                                    .get_public_property("x", activation)?
                                    .coerce_to_number(activation)?;
                                let y = e
                                    .get_public_property("y", activation)?
                                    .coerce_to_number(activation)?;
                                Ok((x, y))
                            } else {
                                Ok((0.0, 0.0))
                            }
                        } else {
                            Ok((0.0, 0.0))
                        }
                    } else {
                        Ok((0.0, 0.0))
                    }
                })
                .collect();
            let octave_offsets = octave_offsets?;

            operations::perlin_noise(
                activation.context.gc_context,
                bitmap_data,
                (base_x, base_y),
                num_octaves,
                seed,
                stitch,
                fractal_noise,
                channel_options,
                grayscale,
                octave_offsets,
            );
        }
    }

    Ok(Value::Undefined)
}

/// Implement `BitmapData.threshold`
pub fn threshold<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(bitmap_data) = this.and_then(|this| this.as_bitmap_data()) {
        if !bitmap_data.disposed() {
            let src_bitmap = args.get_object(activation, 0, "sourceBitmapData")?;
            let source_rect = args.get_object(activation, 1, "sourceRect")?;
            let dest_point = args.get_object(activation, 2, "dstPoint")?;
            let dest_point = (
                dest_point
                    .get_public_property("x", activation)?
                    .coerce_to_i32(activation)?,
                dest_point
                    .get_public_property("y", activation)?
                    .coerce_to_i32(activation)?,
            );
            let operation = args.try_get_string(activation, 3)?;
            let threshold = args.get_u32(activation, 4)?;
            let color = args.get_i32(activation, 5)?;
            let mask = args.get_u32(activation, 6)?;
            let copy_source = args.get_bool(7);

            let operation = if let Some(operation) = operation {
                if let Some(operation) = ThresholdOperation::from_wstr(&operation) {
                    operation
                } else {
                    // It's wrong but this is what Flash says.
                    return Err(Error::AvmError(argument_error(
                        activation,
                        "Parameter 0 is of the incorrect type. Should be type Operation.",
                        2005,
                    )?));
                }
            } else {
                return Err(null_parameter_error(activation, "operation"));
            };

            let src_min_x = source_rect
                .get_public_property("x", activation)?
                .coerce_to_i32(activation)?;
            let src_min_y = source_rect
                .get_public_property("y", activation)?
                .coerce_to_i32(activation)?;
            let src_width = source_rect
                .get_public_property("width", activation)?
                .coerce_to_i32(activation)?;
            let src_height = source_rect
                .get_public_property("height", activation)?
                .coerce_to_i32(activation)?;

            if let Some(src_bitmap) = src_bitmap.as_bitmap_data() {
                src_bitmap.check_valid(activation)?;

                return Ok(operations::threshold(
                    activation.context.gc_context,
                    bitmap_data,
                    src_bitmap,
                    (src_min_x, src_min_y, src_width, src_height),
                    dest_point,
                    operation,
                    threshold,
                    color,
                    mask,
                    copy_source,
                )
                .into());
            }
        }
    }

    Ok(Value::Undefined)
}
