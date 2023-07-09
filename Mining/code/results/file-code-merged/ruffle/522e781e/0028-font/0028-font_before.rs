//! `flash.text.Font` builtin/prototype

use crate::avm2::activation::Activation;
use crate::avm2::class::{Class, ClassAttributes};
use crate::avm2::method::{Method, NativeMethodImpl};
use crate::avm2::object::{Object, TObject};
use crate::avm2::value::Value;
use crate::avm2::Multiname;
use crate::avm2::Namespace;
use crate::avm2::QName;
use crate::avm2::{ArrayObject, ArrayStorage, Error};
use crate::avm2_stub_getter;
use crate::character::Character;
use crate::string::AvmString;
use gc_arena::GcCell;

/// Implements `flash.text.Font`'s instance constructor.
pub fn instance_init<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        activation.super_init(this, &[])?;
    }

    Ok(Value::Undefined)
}

/// Implements `flash.text.Font`'s class constructor.
pub fn class_init<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(Value::Undefined)
}

/// Implements `Font.fontName`
pub fn font_name<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some((movie, character_id)) = this.and_then(|this| this.instance_of()).and_then(|this| {
        activation
            .context
            .library
            .avm2_class_registry()
            .class_symbol(this)
    }) {
        if let Some(Character::Font(font)) = activation
            .context
            .library
            .library_for_movie_mut(movie)
            .character_by_id(character_id)
        {
            return Ok(AvmString::new_utf8(
                activation.context.gc_context,
                font.descriptor().class(),
            )
            .into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `Font.fontStyle`
pub fn font_style<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some((movie, character_id)) = this.and_then(|this| this.instance_of()).and_then(|this| {
        activation
            .context
            .library
            .avm2_class_registry()
            .class_symbol(this)
    }) {
        if let Some(Character::Font(font)) = activation
            .context
            .library
            .library_for_movie_mut(movie)
            .character_by_id(character_id)
        {
            return match (font.descriptor().bold(), font.descriptor().italic()) {
                (false, false) => Ok("regular".into()),
                (false, true) => Ok("italic".into()),
                (true, false) => Ok("bold".into()),
                (true, true) => Ok("boldItalic".into()),
            };
        }
    }

    Ok(Value::Undefined)
}

/// Implements `Font.fontType`
pub fn font_type<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some((movie, character_id)) = this.and_then(|this| this.instance_of()).and_then(|this| {
        activation
            .context
            .library
            .avm2_class_registry()
            .class_symbol(this)
    }) {
        if let Some(Character::Font(_)) = activation
            .context
            .library
            .library_for_movie_mut(movie)
            .character_by_id(character_id)
        {
            //TODO: How do we distinguish between CFF and non-CFF embedded fonts?
            return Ok("embedded".into());
        }
    }

    Ok(Value::Undefined)
}

/// Implements `Font.hasGlyphs`
pub fn has_glyphs<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some((movie, character_id)) = this.and_then(|this| this.instance_of()).and_then(|this| {
        activation
            .context
            .library
            .avm2_class_registry()
            .class_symbol(this)
    }) {
        let my_str = args
            .get(0)
            .cloned()
            .unwrap_or(Value::Undefined)
            .coerce_to_string(activation)?;

        if let Some(Character::Font(font)) = activation
            .context
            .library
            .library_for_movie_mut(movie)
            .character_by_id(character_id)
        {
            return Ok(font.has_glyphs_for_str(&my_str).into());
        }
    }

    Ok(Value::Undefined)
}

/// `Font.enumerateFonts`
pub fn enumerate_fonts<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm2_stub_getter!(activation, "flash.text.Font", "enumerateFonts");
    Ok(ArrayObject::from_storage(activation, ArrayStorage::new(0))?.into())
}

/// `Font.registerFont`
pub fn register_font<'gc>(
    activation: &mut Activation<'_, 'gc>,
    _this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm2_stub_getter!(activation, "flash.text.Font", "registerFont");
    Ok(Value::Undefined)
}

/// Construct `Font`'s class.
pub fn create_class<'gc>(activation: &mut Activation<'_, 'gc>) -> GcCell<'gc, Class<'gc>> {
    let mc = activation.context.gc_context;
    let class = Class::new(
        QName::new(Namespace::package("flash.text", mc), "Font"),
        Some(Multiname::new(activation.avm2().public_namespace, "Object")),
        Method::from_builtin(instance_init, "<Font instance initializer>", mc),
        Method::from_builtin(class_init, "<Font class initializer>", mc),
        mc,
    );

    let mut write = class.write(mc);

    write.set_attributes(ClassAttributes::SEALED);

    const PUBLIC_INSTANCE_PROPERTIES: &[(
        &str,
        Option<NativeMethodImpl>,
        Option<NativeMethodImpl>,
    )] = &[
        ("fontName", Some(font_name), None),
        ("fontStyle", Some(font_style), None),
        ("fontType", Some(font_type), None),
    ];
    write.define_builtin_instance_properties(
        mc,
        activation.avm2().public_namespace,
        PUBLIC_INSTANCE_PROPERTIES,
    );

    const PUBLIC_INSTANCE_METHODS: &[(&str, NativeMethodImpl)] = &[("hasGlyphs", has_glyphs)];
    write.define_builtin_instance_methods(
        mc,
        activation.avm2().public_namespace,
        PUBLIC_INSTANCE_METHODS,
    );

    const PUBLIC_CLASS_METHODS: &[(&str, NativeMethodImpl)] = &[
        ("enumerateFonts", enumerate_fonts),
        ("registerFont", register_font),
    ];
    write.define_builtin_class_methods(
        mc,
        activation.avm2().public_namespace,
        PUBLIC_CLASS_METHODS,
    );

    class
}
