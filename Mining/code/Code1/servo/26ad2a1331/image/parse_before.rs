    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        enum Shape {
            Linear,
            Radial,
        }

        let func = input.expect_function()?;
        let result = match_ignore_ascii_case! { &func,
            "linear-gradient" => {
                Some((Shape::Linear, false, CompatMode::Modern))
            },
            "-webkit-linear-gradient" => {
                Some((Shape::Linear, false, CompatMode::WebKit))
            },
            "repeating-linear-gradient" => {
                Some((Shape::Linear, true, CompatMode::Modern))
            },
            "-webkit-repeating-linear-gradient" => {
                Some((Shape::Linear, true, CompatMode::WebKit))
            },
            "radial-gradient" => {
                Some((Shape::Radial, false, CompatMode::Modern))
            },
            "-webkit-radial-gradient" => {
                Some((Shape::Radial, false, CompatMode::WebKit))
            },
            "repeating-radial-gradient" => {
                Some((Shape::Radial, true, CompatMode::Modern))
            },
            "-webkit-repeating-radial-gradient" => {
                Some((Shape::Radial, true, CompatMode::WebKit))
            },
            "-webkit-gradient" => {
                return input.parse_nested_block(|i| Self::parse_webkit_gradient_argument(context, i));
            },
            _ => None,
        };

        let (shape, repeating, compat_mode) = match result {
            Some(result) => result,
            None => return Err(StyleParseError::UnexpectedFunction(func).into()),
        };

        let (kind, items) = input.parse_nested_block(|i| {
            let shape = match shape {
                Shape::Linear => GradientKind::parse_linear(context, i, compat_mode)?,
                Shape::Radial => GradientKind::parse_radial(context, i, compat_mode)?,
            };
            let items = GradientItem::parse_comma_separated(context, i)?;
            Ok((shape, items))
        })?;

        if items.len() < 2 {
            return Err(StyleParseError::UnspecifiedError.into());
        }

        Ok(Gradient {
            items: items,
            repeating: repeating,
            kind: kind,
            compat_mode: compat_mode,
        })
    }
}

impl Gradient {
    fn parse_webkit_gradient_argument<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                              -> Result<Self, ParseError<'i>> {
        type Point = GenericPosition<Component<X>, Component<Y>>;

        #[derive(Clone, Copy)]
        enum Component<S> {
            Center,
            Number(NumberOrPercentage),
            Side(S),
        }

        impl LineDirection {
            fn from_points(first: Point, second: Point) -> Self {
                let h_ord = first.horizontal.partial_cmp(&second.horizontal);
                let v_ord = first.vertical.partial_cmp(&second.vertical);
                let (h, v) = match (h_ord, v_ord) {
                    (Some(h), Some(v)) => (h, v),
                    _ => return LineDirection::Vertical(Y::Bottom),
                };
                match (h, v) {
                    (Ordering::Less, Ordering::Less) => {
                        LineDirection::Corner(X::Right, Y::Bottom)
                    },
                    (Ordering::Less, Ordering::Equal) => {
                        LineDirection::Horizontal(X::Right)
                    },
                    (Ordering::Less, Ordering::Greater) => {
                        LineDirection::Corner(X::Right, Y::Top)
                    },
                    (Ordering::Equal, Ordering::Greater) => {
                        LineDirection::Vertical(Y::Top)
                    },
                    (Ordering::Equal, Ordering::Equal) |
                    (Ordering::Equal, Ordering::Less) => {
                        LineDirection::Vertical(Y::Bottom)
                    },
                    (Ordering::Greater, Ordering::Less) => {
                        LineDirection::Corner(X::Left, Y::Bottom)
                    },
                    (Ordering::Greater, Ordering::Equal) => {
                        LineDirection::Horizontal(X::Left)
                    },
                    (Ordering::Greater, Ordering::Greater) => {
                        LineDirection::Corner(X::Left, Y::Top)
                    },
                }
            }
        }

        impl From<Point> for Position {
            fn from(point: Point) -> Self {
                Self::new(point.horizontal.into(), point.vertical.into())
            }
        }

        impl Parse for Point {
            fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                             -> Result<Self, ParseError<'i>> {
                input.try(|i| {
                    let x = Component::parse(context, i)?;
                    let y = Component::parse(context, i)?;

                    Ok(Self::new(x, y))
                })
            }
        }

        impl<S: Side> From<Component<S>> for NumberOrPercentage {
            fn from(component: Component<S>) -> Self {
                match component {
                    Component::Center => NumberOrPercentage::Percentage(Percentage(0.5)),
                    Component::Number(number) => number,
                    Component::Side(side) => {
                        let p = Percentage(if side.is_start() { 0. } else { 1. });
                        NumberOrPercentage::Percentage(p)
                    },
                }
            }
        }

        impl<S: Side> From<Component<S>> for PositionComponent<S> {
            fn from(component: Component<S>) -> Self {
                match component {
                    Component::Center => {
                        PositionComponent::Center
                    },
                    Component::Number(NumberOrPercentage::Number(number)) => {
                        PositionComponent::Length(Length::from_px(number.value).into())
                    },
                    Component::Number(NumberOrPercentage::Percentage(p)) => {
                        PositionComponent::Length(p.into())
                    },
                    Component::Side(side) => {
                        PositionComponent::Side(side, None)
                    },
                }
            }
        }

        impl<S: Copy + Side> Component<S> {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                match (NumberOrPercentage::from(*self), NumberOrPercentage::from(*other)) {
                    (NumberOrPercentage::Percentage(a), NumberOrPercentage::Percentage(b)) => {
                        a.0.partial_cmp(&b.0)
                    },
                    (NumberOrPercentage::Number(a), NumberOrPercentage::Number(b)) => {
                        a.value.partial_cmp(&b.value)
                    },
                    (_, _) => {
                        None
                    }
                }
            }
        }

        impl<S: Parse> Parse for Component<S> {
            fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                             -> Result<Self, ParseError<'i>> {
                if let Ok(side) = input.try(|i| S::parse(context, i)) {
                    return Ok(Component::Side(side));
                }
                if let Ok(number) = input.try(|i| NumberOrPercentage::parse(context, i)) {
                    return Ok(Component::Number(number));
                }
                input.try(|i| i.expect_ident_matching("center"))?;
                Ok(Component::Center)
            }
        }

        let ident = input.expect_ident()?;
        input.expect_comma()?;

        let (kind, reverse_stops) = match_ignore_ascii_case! { &ident,
            "linear" => {
                let first = Point::parse(context, input)?;
                input.expect_comma()?;
                let second = Point::parse(context, input)?;

                let direction = LineDirection::from_points(first, second);
                let kind = GenericGradientKind::Linear(direction);

                (kind, false)
            },
            "radial" => {
                let first_point = Point::parse(context, input)?;
                input.expect_comma()?;
                let first_radius = Number::parse(context, input)?;
                input.expect_comma()?;
                let second_point = Point::parse(context, input)?;
                input.expect_comma()?;
                let second_radius = Number::parse(context, input)?;

                let (reverse_stops, point, radius) = if second_radius.value >= first_radius.value {
                    (false, second_point, second_radius)
                } else {
                    (true, first_point, first_radius)
                };

                let shape = GenericEndingShape::Circle(Circle::Radius(Length::from_px(radius.value)));
                let position = point.into();
                let kind = GenericGradientKind::Radial(shape, position);

                (kind, reverse_stops)
            },
            _ => return Err(SelectorParseError::UnexpectedIdent(ident.clone()).into()),
        };

        let mut items = input.try(|i| {
            i.expect_comma()?;
            i.parse_comma_separated(|i| {
                let function = i.expect_function()?;
                let (color, mut p) = i.parse_nested_block(|i| {
                    let p = match_ignore_ascii_case! { &function,
                        "color-stop" => {
                            let p = match NumberOrPercentage::parse(context, i)? {
                                NumberOrPercentage::Number(number) => number.value,
                                NumberOrPercentage::Percentage(p) => p.0,
                            };
                            i.expect_comma()?;
                            p
                        },
                        "from" => 0.,
                        "to" => 1.,
                        _ => return Err(StyleParseError::UnexpectedFunction(function.clone()).into()),
                    };
                    let color = Color::parse(context, i)?;
                    if color == Color::CurrentColor {
                        return Err(StyleParseError::UnspecifiedError.into());
                    }
                    Ok((color.into(), p))
                })?;
                if reverse_stops {
                    p = 1. - p;
                }
                Ok(GenericGradientItem::ColorStop(GenericColorStop {
                    color: color,
                    position: Some(LengthOrPercentage::Percentage(Percentage(p))),
                }))
            })
        }).unwrap_or(vec![]);

        if items.is_empty() {
            items = vec![
                GenericGradientItem::ColorStop(GenericColorStop {
                    color: Color::transparent().into(),
                    position: Some(Percentage(0.).into()),
                }),
                GenericGradientItem::ColorStop(GenericColorStop {
                    color: Color::transparent().into(),
                    position: Some(Percentage(1.).into()),
                }),
            ];
        } else if items.len() == 1 {
            let first = items[0].clone();
            items.push(first);
        } else {
            items.sort_by(|a, b| {
                match (a, b) {
                    (&GenericGradientItem::ColorStop(ref a), &GenericGradientItem::ColorStop(ref b)) => {
                        match (&a.position, &b.position) {
                            (&Some(LengthOrPercentage::Percentage(a)), &Some(LengthOrPercentage::Percentage(b))) => {
                                let ordering = a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal);
                                if ordering != Ordering::Equal {
                                    return ordering;
                                }
                            },
                            _ => {},
                        }
                    },
                    _ => {},
                }
                if reverse_stops {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            })
        }

        Ok(GenericGradient {
            kind: kind,
            items: items,
            repeating: false,
            compat_mode: CompatMode::Modern,
        })
    }
