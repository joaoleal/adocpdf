//! The shape of a page: its size and its margins.

use std::error::Error;
use std::fmt;

use crate::length::Length;

/// The blank border around a page's content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    /// Space above the content.
    pub top: Length,
    /// Space to the right of the content.
    pub right: Length,
    /// Space below the content.
    pub bottom: Length,
    /// Space to the left of the content.
    pub left: Length,
}

impl Margins {
    /// Creates margins with the same length on every side.
    #[must_use]
    pub fn uniform(length: Length) -> Self {
        Self {
            top: length,
            right: length,
            bottom: length,
            left: length,
        }
    }
}

/// A page size and the margins inside it.
///
/// Construction checks that the margins leave something to print on, so any
/// `PageGeometry` in hand describes a page with a usable content area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry {
    width: Length,
    height: Length,
    margins: Margins,
}

impl PageGeometry {
    /// Creates a page geometry.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPageGeometry`] when the horizontal margins consume the
    /// page width, or the vertical margins consume the page height, leaving no
    /// area to print on.
    pub fn new(
        width: Length,
        height: Length,
        margins: Margins,
    ) -> Result<Self, InvalidPageGeometry> {
        let horizontal = margins.left.saturating_add(margins.right);
        if horizontal >= width {
            return Err(InvalidPageGeometry::NoPrintableWidth {
                width_points: width.points(),
                margins_points: horizontal.points(),
            });
        }

        let vertical = margins.top.saturating_add(margins.bottom);
        if vertical >= height {
            return Err(InvalidPageGeometry::NoPrintableHeight {
                height_points: height.points(),
                margins_points: vertical.points(),
            });
        }

        Ok(Self {
            width,
            height,
            margins,
        })
    }

    /// The full page width, margins included.
    #[must_use]
    pub fn width(&self) -> Length {
        self.width
    }

    /// The full page height, margins included.
    #[must_use]
    pub fn height(&self) -> Length {
        self.height
    }

    /// The margins inside the page.
    #[must_use]
    pub fn margins(&self) -> Margins {
        self.margins
    }
}

/// Why a proposed page geometry is not usable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InvalidPageGeometry {
    /// The left and right margins leave no width to print on.
    NoPrintableWidth {
        /// The page width, in points.
        width_points: f64,
        /// The left and right margins combined, in points.
        margins_points: f64,
    },
    /// The top and bottom margins leave no height to print on.
    NoPrintableHeight {
        /// The page height, in points.
        height_points: f64,
        /// The top and bottom margins combined, in points.
        margins_points: f64,
    },
}

impl fmt::Display for InvalidPageGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPrintableWidth {
                width_points,
                margins_points,
            } => write!(
                f,
                "left and right margins total {margins_points}pt, which leaves no printable \
                 width on a page {width_points}pt wide"
            ),
            Self::NoPrintableHeight {
                height_points,
                margins_points,
            } => write!(
                f,
                "top and bottom margins total {margins_points}pt, which leaves no printable \
                 height on a page {height_points}pt tall"
            ),
        }
    }
}

impl Error for InvalidPageGeometry {}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(value: f64) -> Length {
        Length::from_points(value).unwrap()
    }

    fn a4() -> PageGeometry {
        PageGeometry::new(
            Length::from_millimeters(210.0).unwrap(),
            Length::from_millimeters(297.0).unwrap(),
            Margins::uniform(Length::from_millimeters(20.0).unwrap()),
        )
        .unwrap()
    }

    #[test]
    fn a_page_with_room_to_print_is_accepted() {
        let geometry = a4();

        assert!((geometry.width().millimeters() - 210.0).abs() < 1e-9);
        assert!((geometry.height().millimeters() - 297.0).abs() < 1e-9);
    }

    #[test]
    fn margins_wider_than_the_page_are_rejected() {
        let error = PageGeometry::new(points(100.0), points(200.0), Margins::uniform(points(60.0)))
            .unwrap_err();

        assert_eq!(
            error,
            InvalidPageGeometry::NoPrintableWidth {
                width_points: 100.0,
                margins_points: 120.0,
            }
        );
    }

    #[test]
    fn margins_exactly_filling_the_width_are_rejected() {
        let error = PageGeometry::new(points(100.0), points(200.0), Margins::uniform(points(50.0)))
            .unwrap_err();

        assert!(matches!(
            error,
            InvalidPageGeometry::NoPrintableWidth { .. }
        ));
    }

    #[test]
    fn margins_taller_than_the_page_are_rejected() {
        let margins = Margins {
            top: points(80.0),
            right: points(10.0),
            bottom: points(80.0),
            left: points(10.0),
        };

        let error = PageGeometry::new(points(100.0), points(150.0), margins).unwrap_err();

        assert_eq!(
            error,
            InvalidPageGeometry::NoPrintableHeight {
                height_points: 150.0,
                margins_points: 160.0,
            }
        );
    }

    #[test]
    fn rejection_names_the_offending_setting() {
        let error = PageGeometry::new(points(100.0), points(200.0), Margins::uniform(points(60.0)))
            .unwrap_err();

        let message = error.to_string();

        assert!(
            message.contains("left and right margins"),
            "message must name which margins are at fault, got: {message}"
        );
        assert!(
            message.contains("120") && message.contains("100"),
            "message must report the measurements that conflict, got: {message}"
        );
    }

    #[test]
    fn uniform_margins_apply_to_every_side() {
        let margins = Margins::uniform(points(15.0));

        assert_eq!(margins.top, margins.right);
        assert_eq!(margins.right, margins.bottom);
        assert_eq!(margins.bottom, margins.left);
    }
}
