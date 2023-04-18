    fn to_gecko_message(&self) -> (Option<&'static [u8]>, &'static [u8], Action) {
        let (msg, action): (&[u8], Action) = match *self {
            ContextualParseError::UnsupportedPropertyDeclaration(
                _, ParseError { kind: ParseErrorKind::Basic(BasicParseErrorKind::UnexpectedToken(_)), .. }
            ) |
            ContextualParseError::UnsupportedPropertyDeclaration(
                _, ParseError { kind: ParseErrorKind::Basic(BasicParseErrorKind::AtRuleInvalid(_)), .. }
            ) => {
                (b"PEParseDeclarationDeclExpected\0", Action::Skip)
            }
            ContextualParseError::UnsupportedPropertyDeclaration(
                _, ParseError { kind: ParseErrorKind::Custom(ref err), .. }
            ) => {
                match *err {
                    StyleParseErrorKind::InvalidColor(_, _) => {
                        return (Some(b"PEColorNotColor\0"),
                                b"PEValueParsingError\0", Action::Drop)
                    }
                    StyleParseErrorKind::InvalidFilter(_, _) => {
                        return (Some(b"PEExpectedNoneOrURLOrFilterFunction\0"),
                                b"PEValueParsingError\0", Action::Drop)
                    }
                    StyleParseErrorKind::OtherInvalidValue(_) => {
                        (b"PEValueParsingError\0", Action::Drop)
                    }
                    _ => (b"PEUnknownProperty\0", Action::Drop)
                }
            }
            ContextualParseError::UnsupportedPropertyDeclaration(..) =>
                (b"PEUnknownProperty\0", Action::Drop),
            ContextualParseError::UnsupportedFontFaceDescriptor(..) =>
                (b"PEUnknownFontDesc\0", Action::Skip),
            ContextualParseError::InvalidKeyframeRule(..) =>
                (b"PEKeyframeBadName\0", Action::Nothing),
            ContextualParseError::UnsupportedKeyframePropertyDeclaration(..) =>
                (b"PEBadSelectorKeyframeRuleIgnored\0", Action::Nothing),
            ContextualParseError::InvalidRule(
                _, ParseError { kind: ParseErrorKind::Custom(
                    StyleParseErrorKind::UnexpectedTokenWithinNamespace(_)
                ), .. }
            ) => {
                (b"PEAtNSUnexpected\0", Action::Nothing)
            }
            ContextualParseError::InvalidRule(
                _, ParseError { kind: ParseErrorKind::Basic(BasicParseErrorKind::AtRuleInvalid(_)), .. }
            ) |
            ContextualParseError::InvalidRule(
                _, ParseError { kind: ParseErrorKind::Custom(
                    StyleParseErrorKind::UnsupportedAtRule(_)
                ), .. }
            ) => {
                (b"PEUnknownAtRule\0", Action::Nothing)
            }
            ContextualParseError::InvalidRule(_, ref err) => {
                let prefix = match err.kind {
                    ParseErrorKind::Custom(StyleParseErrorKind::SelectorError(ref err)) => match *err {
                        SelectorParseErrorKind::UnexpectedTokenInAttributeSelector(_) => {
                            Some(&b"PEAttSelUnexpected\0"[..])
                        }
                        SelectorParseErrorKind::ExpectedBarInAttr(_) => {
                            Some(&b"PEAttSelNoBar\0"[..])
                        }
                        SelectorParseErrorKind::BadValueInAttr(_) => {
                            Some(&b"PEAttSelBadValue\0"[..])
                        }
                        SelectorParseErrorKind::NoQualifiedNameInAttributeSelector(_) => {
                            Some(&b"PEAttributeNameOrNamespaceExpected\0"[..])
                        }
                        SelectorParseErrorKind::InvalidQualNameInAttr(_) => {
                            Some(&b"PEAttributeNameExpected\0"[..])
                        }
                        SelectorParseErrorKind::ExplicitNamespaceUnexpectedToken(_) => {
                            Some(&b"PETypeSelNotType\0"[..])
                        }
                        SelectorParseErrorKind::ExpectedNamespace(_) => {
                           Some(&b"PEUnknownNamespacePrefix\0"[..])
                        }
                        SelectorParseErrorKind::EmptySelector => {
                            Some(&b"PESelectorGroupNoSelector\0"[..])
                        }
                        SelectorParseErrorKind::DanglingCombinator => {
                            Some(&b"PESelectorGroupExtraCombinator\0"[..])
                        }
                        SelectorParseErrorKind::UnsupportedPseudoClassOrElement(_) => {
                            Some(&b"PEPseudoSelUnknown\0"[..])
                        }
                        SelectorParseErrorKind::PseudoElementExpectedColon(_) => {
                            Some(&b"PEPseudoSelEndOrUserActionPC\0"[..])
                        }
                        SelectorParseErrorKind::NoIdentForPseudo(_) => {
                            Some(&b"PEPseudoClassArgNotIdent\0"[..])
                        }
                        SelectorParseErrorKind::PseudoElementExpectedIdent(_) => {
                            Some(&b"PEPseudoSelBadName\0"[..])
                        }
                        SelectorParseErrorKind::ClassNeedsIdent(_) => {
                            Some(&b"PEClassSelNotIdent\0"[..])
                        }
                        SelectorParseErrorKind::EmptyNegation => {
                            Some(&b"PENegationBadArg\0"[..])
                        }
                        _ => None,
                    },
                    _ => None,
                };
                return (prefix, b"PEBadSelectorRSIgnored\0", Action::Nothing);
            }
            ContextualParseError::InvalidMediaRule(_, ref err) => {
                let err: &[u8] = match err.kind {
                    ParseErrorKind::Custom(StyleParseErrorKind::ExpectedIdentifier(..)) => {
                        b"PEGatherMediaNotIdent\0"
                    },
                    ParseErrorKind::Custom(StyleParseErrorKind::MediaQueryExpectedFeatureName(..)) => {
                        b"PEMQExpectedFeatureName\0"
                    },
                    ParseErrorKind::Custom(StyleParseErrorKind::MediaQueryExpectedFeatureValue) => {
                        b"PEMQExpectedFeatureValue\0"
                    },
                    ParseErrorKind::Custom(StyleParseErrorKind::RangedExpressionWithNoValue) => {
                        b"PEMQNoMinMaxWithoutValue\0"
                    },
                    _ => {
                        b"PEDeclDropped\0"
                    },
                };
                (err, Action::Nothing)
            }
            ContextualParseError::UnsupportedRule(..) =>
                (b"PEDeclDropped\0", Action::Nothing),
            ContextualParseError::UnsupportedViewportDescriptorDeclaration(..) |
            ContextualParseError::UnsupportedCounterStyleDescriptorDeclaration(..) |
            ContextualParseError::InvalidCounterStyleWithoutSymbols(..) |
            ContextualParseError::InvalidCounterStyleNotEnoughSymbols(..) |
            ContextualParseError::InvalidCounterStyleWithoutAdditiveSymbols |
            ContextualParseError::InvalidCounterStyleExtendsWithSymbols |
            ContextualParseError::InvalidCounterStyleExtendsWithAdditiveSymbols |
            ContextualParseError::UnsupportedFontFeatureValuesDescriptor(..) |
            ContextualParseError::InvalidFontFeatureValuesRule(..) =>
                (b"PEUnknownAtRule\0", Action::Skip),
            ContextualParseError::UnsupportedValue(_, ParseError { ref kind, .. }) => {
                match *kind {
                    ParseErrorKind::Custom(
                        StyleParseErrorKind::ValueError(
                            ValueParseErrorKind::InvalidColor(..)
                        )
                    ) => (b"PEColorNotColor\0", Action::Nothing),
                    _ => {
                        // Not the best error message, since we weren't parsing
                        // a declaration, just a value. But we don't produce
                        // UnsupportedValue errors other than InvalidColors
                        // currently.
                        debug_assert!(false, "should use a more specific error message");
                        (b"PEDeclDropped\0", Action::Nothing)
                    }
                }
            }
        };
        (None, msg, action)
    }
