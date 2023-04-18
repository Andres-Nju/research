    pub fn initial_values() -> &'static Self {
        unsafe {
            debug_assert!(!INITIAL_GECKO_VALUES.is_null());
            &*INITIAL_GECKO_VALUES
        }
    }

    pub unsafe fn initialize() {
        debug_assert!(INITIAL_GECKO_VALUES.is_null());
        INITIAL_GECKO_VALUES = Box::into_raw(Box::new(ComputedValues {
            % for style_struct in data.style_structs:
               ${style_struct.ident}: style_structs::${style_struct.name}::initial(),
            % endfor
            custom_properties: None,
            shareable: true,
            writing_mode: WritingMode::empty(),
            root_font_size: longhands::font_size::get_initial_value(),
        }));
    }

    pub unsafe fn shutdown() {
        debug_assert!(!INITIAL_GECKO_VALUES.is_null());
        let _ = Box::from_raw(INITIAL_GECKO_VALUES);
    }

    #[inline]
    pub fn do_cascade_property<F: FnOnce(&[CascadePropertyFn])>(f: F) {
        f(&CASCADE_PROPERTY)
    }

    % for style_struct in data.style_structs:
    #[inline]
    pub fn clone_${style_struct.name_lower}(&self) -> Arc<style_structs::${style_struct.name}> {
        self.${style_struct.ident}.clone()
    }
    #[inline]
    pub fn get_${style_struct.name_lower}(&self) -> &style_structs::${style_struct.name} {
        &self.${style_struct.ident}
    }
    #[inline]
    pub fn mutate_${style_struct.name_lower}(&mut self) -> &mut style_structs::${style_struct.name} {
        Arc::make_mut(&mut self.${style_struct.ident})
    }
    % endfor

    pub fn custom_properties(&self) -> Option<Arc<ComputedValuesMap>> {
        self.custom_properties.as_ref().map(|x| x.clone())
    }

    pub fn root_font_size(&self) -> Au { self.root_font_size }
    pub fn set_root_font_size(&mut self, s: Au) { self.root_font_size = s; }
    pub fn set_writing_mode(&mut self, mode: WritingMode) { self.writing_mode = mode; }

    // FIXME(bholley): Implement this properly.
    #[inline]
    pub fn is_multicol(&self) -> bool { false }
}

<%def name="declare_style_struct(style_struct)">
pub struct ${style_struct.gecko_struct_name} {
    gecko: ${style_struct.gecko_ffi_name},
}
</%def>

<%def name="impl_simple_setter(ident, gecko_ffi_name)">
    #[allow(non_snake_case)]
    pub fn set_${ident}(&mut self, v: longhands::${ident}::computed_value::T) {
        ${set_gecko_property(gecko_ffi_name, "v")}
    }
</%def>

<%def name="impl_simple_clone(ident, gecko_ffi_name)">
    #[allow(non_snake_case)]
    pub fn clone_${ident}(&self) -> longhands::${ident}::computed_value::T {
        self.gecko.${gecko_ffi_name}
    }
</%def>

<%def name="impl_simple_copy(ident, gecko_ffi_name, *kwargs)">
    #[allow(non_snake_case)]
    pub fn copy_${ident}_from(&mut self, other: &Self) {
        self.gecko.${gecko_ffi_name} = other.gecko.${gecko_ffi_name};
    }
</%def>

<%def name="impl_coord_copy(ident, gecko_ffi_name)">
    #[allow(non_snake_case)]
    pub fn copy_${ident}_from(&mut self, other: &Self) {
        self.gecko.${gecko_ffi_name}.copy_from(&other.gecko.${gecko_ffi_name});
    }
</%def>

<%!
def is_border_style_masked(ffi_name):
    return ffi_name.split("[")[0] in ["mBorderStyle", "mOutlineStyle", "mTextDecorationStyle"]

def get_gecko_property(ffi_name):
    if is_border_style_masked(ffi_name):
        return "(self.gecko.%s & (structs::BORDER_STYLE_MASK as u8))" % ffi_name
    return "self.gecko.%s" % ffi_name

def set_gecko_property(ffi_name, expr):
    if is_border_style_masked(ffi_name):
        return "self.gecko.%s &= !(structs::BORDER_STYLE_MASK as u8);" % ffi_name + \
               "self.gecko.%s |= %s as u8;" % (ffi_name, expr)
    elif ffi_name == "__LIST_STYLE_TYPE__":
        return "unsafe { Gecko_SetListStyleType(&mut self.gecko, %s as u32); }" % expr
    return "self.gecko.%s = %s;" % (ffi_name, expr)
%>

<%def name="impl_keyword_setter(ident, gecko_ffi_name, keyword)">
    #[allow(non_snake_case)]
    pub fn set_${ident}(&mut self, v: longhands::${ident}::computed_value::T) {
        use properties::longhands::${ident}::computed_value::T as Keyword;
        // FIXME(bholley): Align binary representations and ditch |match| for cast + static_asserts
        let result = match v {
            % for value in keyword.values_for('gecko'):
                % if keyword.needs_cast():
                    Keyword::${to_rust_ident(value)} => structs::${keyword.gecko_constant(value)} as u8,
                % else:
                    Keyword::${to_rust_ident(value)} => structs::${keyword.gecko_constant(value)},
                % endif
            % endfor
        };
        ${set_gecko_property(gecko_ffi_name, "result")}
    }
</%def>

<%def name="impl_keyword_clone(ident, gecko_ffi_name, keyword)">
    #[allow(non_snake_case)]
    pub fn clone_${ident}(&self) -> longhands::${ident}::computed_value::T {
        use properties::longhands::${ident}::computed_value::T as Keyword;
        // FIXME(bholley): Align binary representations and ditch |match| for cast + static_asserts
        match ${get_gecko_property(gecko_ffi_name)} as u32 {
            % for value in keyword.values_for('gecko'):
            structs::${keyword.gecko_constant(value)} => Keyword::${to_rust_ident(value)},
            % endfor
            x => panic!("Found unexpected value in style struct for ${ident} property: {}", x),
        }
    }
</%def>

<%def name="clear_color_flags(color_flags_ffi_name)">
    % if color_flags_ffi_name:
    self.gecko.${color_flags_ffi_name} &= !(structs::BORDER_COLOR_SPECIAL as u8);
    % endif
</%def>

<%def name="set_current_color_flag(color_flags_ffi_name)">
    % if color_flags_ffi_name:
    self.gecko.${color_flags_ffi_name} |= structs::BORDER_COLOR_FOREGROUND as u8;
    % else:
    // FIXME(heycam): This is a Gecko property that doesn't store currentColor
    // as a computed value.  These are currently handled by converting
    // currentColor to the current value of the color property at computed
    // value time, but we don't have access to the Color struct here.
    // In the longer term, Gecko should store currentColor as a computed
    // value, so that we don't need to do this:
    // https://bugzilla.mozilla.org/show_bug.cgi?id=760345
    unimplemented!();
    % endif
</%def>

<%def name="get_current_color_flag_from(field)">
    (${field} & (structs::BORDER_COLOR_FOREGROUND as u8)) != 0
