    fn enabled_for_all_content(self) -> bool {
        ${static_non_custom_property_id_set(
            "EXPERIMENTAL",
            lambda p: p.experimental(engine)
        )}

        ${static_non_custom_property_id_set(
            "ALWAYS_ENABLED",
            lambda p: (not p.experimental(engine)) and p.enabled_in_content()
        )}

        let passes_pref_check = || {
            % if engine == "gecko":
                unsafe { structs::nsCSSProps_gPropertyEnabled[self.0] }
            % else:
                static PREF_NAME: [Option< &str>; ${len(data.longhands) + len(data.shorthands)}] = [
                    % for property in data.longhands + data.shorthands:
                        <%
                            attrs = {"servo-2013": "servo_2013_pref", "servo-2020": "servo_2020_pref"}
                            pref = getattr(property, attrs[engine])
                        %>
                        % if pref:
                            Some("${pref}"),
                        % else:
                            None,
                        % endif
                    % endfor
                ];
                let pref = match PREF_NAME[self.0] {
                    None => return true,
                    Some(pref) => pref,
                };

                prefs::pref_map().get(pref).as_bool().unwrap_or(false)
            % endif
        };

        if ALWAYS_ENABLED.contains(self) {
            return true
        }

        if EXPERIMENTAL.contains(self) && passes_pref_check() {
            return true
        }

        false
    }
