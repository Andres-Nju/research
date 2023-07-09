use crate::avm2::activation::Activation;
use crate::avm2::bytearray::{Endian, ObjectEncoding};
pub use crate::avm2::object::byte_array_allocator;
use crate::avm2::object::{Object, TObject};
use crate::avm2::value::Value;
use crate::avm2::Error;
use crate::character::Character;
use crate::string::AvmString;
use encoding_rs::Encoding;
use encoding_rs::UTF_8;
use flash_lso::amf0::read::AMF0Decoder;
use flash_lso::amf3::read::AMF3Decoder;
use flash_lso::types::{AMFVersion, Element};

/// Implements `flash.utils.ByteArray`'s instance constructor.
pub fn init<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        activation.super_init(this, &[])?;

        let class_object = this
            .instance_of()
            .ok_or("Attempted to construct ByteArray on a bare object")?;
        if let Some((movie, id)) = activation
            .context
            .library
            .avm2_class_registry()
            .class_symbol(class_object)
        {
            if let Some(lib) = activation.context.library.library_for_movie(movie) {
                if let Some(Character::BinaryData(binary_data)) = lib.character_by_id(id) {
                    let mut byte_array = this
                        .as_bytearray_mut(activation.context.gc_context)
                        .ok_or_else(|| "Unable to get bytearray storage".to_string())?;
                    byte_array.clear();
                    byte_array.write_bytes(binary_data.as_ref())?;
                    byte_array.set_position(0);
                }
            }
        }
    }

    Ok(Value::Undefined)
}

/// Writes a single byte to the bytearray
pub fn write_byte<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let byte = args
                .get(0)
                .cloned()
                .unwrap_or(Value::Undefined)
                .coerce_to_i32(activation)?;
            bytearray.write_bytes(&[byte as u8])?;
        }
    }

    Ok(Value::Undefined)
}

/// Writes multiple bytes to the bytearray from another bytearray
pub fn write_bytes<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        let bytearray = args
            .get(0)
            .unwrap_or(&Value::Undefined)
            .coerce_to_object(activation)?;
        let offset = args
            .get(1)
            .unwrap_or(&Value::Integer(0))
            .coerce_to_u32(activation)? as usize;
        let length = args
            .get(2)
            .unwrap_or(&Value::Integer(0))
            .coerce_to_u32(activation)? as usize;
        if !Object::ptr_eq(this, bytearray) {
            // The ByteArray we are reading from is different than the ByteArray we are writing to,
            // so we are allowed to borrow both at the same time without worrying about a panic

            let ba_read = bytearray
                .as_bytearray()
                .ok_or("ArgumentError: Parameter must be a bytearray")?;
            let to_write = ba_read
                .read_at(
                    // If length is 0, lets read the remaining bytes of ByteArray from the supplied offset
                    if length != 0 {
                        length
                    } else {
                        ba_read.len().saturating_sub(offset)
                    },
                    offset,
                )
                .map_err(|e| e.to_avm(activation))?;

            if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
                bytearray.write_bytes(to_write)?;
            }
        } else if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            // The ByteArray we are reading from is the same as the ByteArray we are writing to,
            // so we only need to borrow once, and we can use `write_bytes_within` to write bytes from our own ByteArray
            let amnt = if length != 0 {
                length
            } else {
                bytearray.len().saturating_sub(offset)
            };
            bytearray.write_bytes_within(offset, amnt)?;
        }
    }

    Ok(Value::Undefined)
}

// Reads the bytes from the current bytearray into another bytearray
pub fn read_bytes<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        let bytearray = args
            .get(0)
            .unwrap_or(&Value::Undefined)
            .coerce_to_object(activation)?;
        let offset = args
            .get(1)
            .unwrap_or(&Value::Integer(0))
            .coerce_to_u32(activation)? as usize;
        let length = args
            .get(2)
            .unwrap_or(&Value::Integer(0))
            .coerce_to_u32(activation)? as usize;

        if !Object::ptr_eq(this, bytearray) {
            if let Some(bytearray_read) = this.as_bytearray() {
                let to_write = bytearray_read
                    .read_bytes(
                        // If length is 0, lets read the remaining bytes of ByteArray
                        if length != 0 {
                            length
                        } else {
                            bytearray_read.bytes_available()
                        },
                    )
                    .map_err(|e| e.to_avm(activation))?;

                let mut ba_write = bytearray
                    .as_bytearray_mut(activation.context.gc_context)
                    .ok_or("ArgumentError: Parameter must be a bytearray")?;

                ba_write.write_at(to_write, offset)?;
            }
        } else if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let amnt = if length != 0 {
                length
            } else {
                bytearray.bytes_available()
            };
            let pos = bytearray.position();
            bytearray.write_at_within(pos, amnt, offset)?;
        }
    }
    Ok(Value::Undefined)
}
pub fn write_utf<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            if let Some(utf_string) = args.get(0) {
                let utf_string = utf_string.coerce_to_string(activation)?;
                // NOTE: there is a bug on old Flash Player (e.g. v11.3); if the string to
                // write ends with an unpaired high surrogate, the routine bails out and nothing
                // is written.
                // The bug is fixed on newer FP versions (e.g. v32), but the fix isn't SWF-version-gated.
                bytearray.write_utf(&utf_string.to_utf8_lossy())?;
            }
        }
    }

    Ok(Value::Undefined)
}

pub fn read_utf<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(AvmString::new_utf8_bytes(
                activation.context.gc_context,
                bytearray.read_utf().map_err(|e| e.to_avm(activation))?,
            )
            .into());
        }
    }

    Ok(Value::Undefined)
}
pub fn to_string<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(
                AvmString::new_utf8_bytes(activation.context.gc_context, bytearray.bytes()).into(),
            );
        }
    }

    Ok(Value::Undefined)
}

pub fn clear<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            bytearray.clear();
            bytearray.shrink_to_fit();
        }
    }

    Ok(Value::Undefined)
}

pub fn get_position<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray.position().into());
        }
    }

    Ok(Value::Undefined)
}

pub fn set_position<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            let num = args
                .get(0)
                .unwrap_or(&Value::Integer(0))
                .coerce_to_u32(activation)?;
            bytearray.set_position(num as usize);
        }
    }

    Ok(Value::Undefined)
}

pub fn get_bytes_available<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray.bytes_available().into());
        }
    }

    Ok(Value::Undefined)
}

pub fn get_length<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray.len().into());
        }
    }

    Ok(Value::Undefined)
}

pub fn set_length<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let len = args
                .get(0)
                .unwrap_or(&Value::Integer(0))
                .coerce_to_u32(activation)? as usize;
            bytearray.set_length(len);
        }
    }

    Ok(Value::Undefined)
}

pub fn get_endian<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(match bytearray.endian() {
                Endian::Big => "bigEndian".into(),
                Endian::Little => "littleEndian".into(),
            });
        }
    }

    Ok(Value::Undefined)
}

pub fn set_endian<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let endian = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_string(activation)?;
            if &endian == b"bigEndian" {
                bytearray.set_endian(Endian::Big);
            } else if &endian == b"littleEndian" {
                bytearray.set_endian(Endian::Little);
            } else {
                return Err("Parameter type must be one of the accepted values.".into());
            }
        }
    }

    Ok(Value::Undefined)
}

pub fn read_short<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_short()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_unsigned_short<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_unsigned_short()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_double<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_double()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_float<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_float()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_int<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_int()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_unsigned_int<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_unsigned_int()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_boolean<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_boolean()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_byte<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_byte()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_utf_bytes<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            let len = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_u32(activation)?;
            return Ok(AvmString::new_utf8(
                activation.context.gc_context,
                String::from_utf8_lossy(
                    bytearray
                        .read_bytes(len as usize)
                        .map_err(|e| e.to_avm(activation))?,
                ),
            )
            .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn read_unsigned_byte<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok(bytearray
                .read_unsigned_byte()
                .map_err(|e| e.to_avm(activation))?
                .into());
        }
    }

    Ok(Value::Undefined)
}

pub fn write_float<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let num = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_number(activation)?;
            bytearray.write_float(num as f32)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn write_double<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let num = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_number(activation)?;
            bytearray.write_double(num)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn write_boolean<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let num = args.get(0).unwrap_or(&Value::Undefined).coerce_to_boolean();
            bytearray.write_boolean(num)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn write_int<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let num = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_i32(activation)?;
            bytearray.write_int(num)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn write_unsigned_int<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let num = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_u32(activation)?;
            bytearray.write_unsigned_int(num)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn write_short<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let num = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_i32(activation)?;
            bytearray.write_short(num as i16)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn write_multi_byte<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let string = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_string(activation)?;
            let charset_label = args
                .get(1)
                .unwrap_or(&"UTF-8".into())
                .coerce_to_string(activation)?;
            let encoder =
                Encoding::for_label(charset_label.to_utf8_lossy().as_bytes()).unwrap_or(UTF_8);
            let utf8 = string.to_utf8_lossy();
            let (encoded_bytes, _, _) = encoder.encode(&utf8);
            bytearray.write_bytes(&encoded_bytes)?;
        }
    }

    Ok(Value::Undefined)
}

pub fn read_multi_byte<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            let len = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_u32(activation)?;
            let charset_label = args
                .get(1)
                .unwrap_or(&"UTF-8".into())
                .coerce_to_string(activation)?;
            let bytes = bytearray
                .read_bytes(len as usize)
                .map_err(|e| e.to_avm(activation))?;
            let encoder =
                Encoding::for_label(charset_label.to_utf8_lossy().as_bytes()).unwrap_or(UTF_8);
            let (decoded_str, _, _) = encoder.decode(bytes);
            return Ok(AvmString::new_utf8(activation.context.gc_context, decoded_str).into());
        }
    }

    Ok(Value::Undefined)
}

pub fn write_utf_bytes<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let string = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_string(activation)?;
            bytearray.write_bytes(string.to_utf8_lossy().as_bytes())?;
        }
    }

    Ok(Value::Undefined)
}

pub fn compress<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let algorithm = args
                .get(0)
                .unwrap_or(&"zlib".into())
                .coerce_to_string(activation)?;
            let algorithm = match algorithm.parse() {
                Ok(algorithm) => algorithm,
                Err(_) => {
                    return Err(Error::AvmError(crate::avm2::error::io_error(
                        activation,
                        "Error #2058: There was an error decompressing the data.",
                        2058,
                    )?))
                }
            };
            let buffer = bytearray.compress(algorithm);
            bytearray.clear();
            bytearray.write_bytes(&buffer)?;
            bytearray.set_position(bytearray.len());
        }
    }

    Ok(Value::Undefined)
}

pub fn uncompress<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let algorithm = args
                .get(0)
                .unwrap_or(&"zlib".into())
                .coerce_to_string(activation)?;
            let algorithm = match algorithm.parse() {
                Ok(algorithm) => algorithm,
                Err(_) => {
                    return Err(Error::AvmError(crate::avm2::error::io_error(
                        activation,
                        "Error #2058: There was an error decompressing the data.",
                        2058,
                    )?))
                }
            };
            let buffer = match bytearray.decompress(algorithm) {
                Some(buffer) => buffer,
                None => {
                    return Err(Error::AvmError(crate::avm2::error::io_error(
                        activation,
                        "Error #2058: There was an error decompressing the data.",
                        2058,
                    )?))
                }
            };
            bytearray.clear();
            bytearray.write_bytes(&buffer)?;
            bytearray.set_position(0);
        }
    }

    Ok(Value::Undefined)
}

pub fn read_object<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            let bytes = bytearray
                .read_at(bytearray.bytes_available(), bytearray.position())
                .map_err(|e| e.to_avm(activation))?;
            let (bytes_left, value) = match bytearray.object_encoding() {
                ObjectEncoding::Amf0 => {
                    let mut decoder = AMF0Decoder::default();
                    let (extra, amf) = decoder
                        .parse_single_element(bytes)
                        .map_err(|_| "Error: Invalid object")?;
                    (
                        extra.len(),
                        crate::avm2::amf::deserialize_value(activation, &amf)?,
                    )
                }
                ObjectEncoding::Amf3 => {
                    let mut decoder = AMF3Decoder::default();
                    let (extra, amf) = decoder
                        .parse_single_element(bytes)
                        .map_err(|_| "Error: Invalid object")?;
                    (
                        extra.len(),
                        crate::avm2::amf::deserialize_value(activation, &amf)?,
                    )
                }
            };

            bytearray.set_position(bytearray.len() - bytes_left);
            return Ok(value);
        }
    }

    Ok(Value::Undefined)
}

pub fn write_object<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let obj = args.get(0).cloned().unwrap_or(Value::Undefined);
            let amf_version = match bytearray.object_encoding() {
                ObjectEncoding::Amf0 => AMFVersion::AMF0,
                ObjectEncoding::Amf3 => AMFVersion::AMF3,
            };
            if let Some(amf) = crate::avm2::amf::serialize_value(activation, obj, amf_version) {
                let element = Element::new("", amf);
                let mut lso = flash_lso::types::Lso::new(vec![element], "", amf_version);
                let bytes = flash_lso::write::write_to_bytes(&mut lso)
                    .map_err(|_| "Failed to serialize object")?;
                // This is kind of hacky: We need to strip out the header and any padding so that we only write
                // the value. In the future, there should be a method to do this in the flash_lso crate.
                let element_padding = match amf_version {
                    AMFVersion::AMF0 => 8,
                    AMFVersion::AMF3 => 7,
                };
                bytearray.write_bytes(
                    &bytes[flash_lso::write::header_length(&lso.header) + element_padding
                        ..bytes.len() - 1],
                )?;
            }
        }
    }
    Ok(Value::Undefined)
}

pub fn get_object_encoding<'gc>(
    _activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(bytearray) = this.as_bytearray() {
            return Ok((bytearray.object_encoding() as u8).into());
        }
    }

    Ok(Value::Undefined)
}

pub fn set_object_encoding<'gc>(
    activation: &mut Activation<'_, 'gc>,
    this: Option<Object<'gc>>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    if let Some(this) = this {
        if let Some(mut bytearray) = this.as_bytearray_mut(activation.context.gc_context) {
            let new_encoding = args
                .get(0)
                .unwrap_or(&Value::Undefined)
                .coerce_to_u32(activation)?;
            match new_encoding {
                0 => bytearray.set_object_encoding(ObjectEncoding::Amf0),
                3 => bytearray.set_object_encoding(ObjectEncoding::Amf3),
                _ => return Err("Parameter type must be one of the accepted values.".into()),
            }
        }
    }

    Ok(Value::Undefined)
}
