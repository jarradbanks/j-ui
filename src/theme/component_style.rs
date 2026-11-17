//! Scoped presentation variants for comparing the component reference styles.

use std::cell::Cell;

/// The reference page's surface and density variants. Soft remains the library default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ComponentStyle {
    Current,
    Line,
    #[default]
    Soft,
}

thread_local! {
    static CURRENT: Cell<ComponentStyle> = const { Cell::new(ComponentStyle::Soft) };
}

pub(crate) fn current() -> ComponentStyle {
    CURRENT.get()
}

/// Applies a component presentation only while painting `contents`, then restores it.
pub fn with_component_style<R>(style: ComponentStyle, contents: impl FnOnce() -> R) -> R {
    struct Restore(ComponentStyle);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.set(self.0);
        }
    }
    let _restore = Restore(CURRENT.replace(style));
    contents()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{radius, size};

    #[test]
    fn nested_presentations_restore_soft_even_after_unwinding() {
        assert_eq!(size::control(), size::CONTROL);
        with_component_style(ComponentStyle::Current, || {
            assert_eq!(size::control(), 24.0);
            with_component_style(ComponentStyle::Line, || {
                assert_eq!(size::control(), 36.0);
                assert_eq!(radius::control(), 8);
            });
            assert_eq!(size::control(), 24.0);
        });
        let result = std::panic::catch_unwind(|| {
            with_component_style(ComponentStyle::Current, || panic!("aborted paint"));
        });
        assert!(result.is_err());
        assert_eq!(current(), ComponentStyle::Soft);
    }
}
