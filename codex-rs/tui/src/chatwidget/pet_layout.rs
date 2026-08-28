//! Transcript layout shared by Full terminal pets and the Coding composition.

use super::ChatWidget;

impl ChatWidget {
    pub(super) fn ambient_pet_wrap_reserved_cols(&self) -> u16 {
        #[cfg(feature = "full-runtime-extensions")]
        {
            self.ambient_pet
                .as_ref()
                .filter(|pet| pet.image_enabled())
                .map(|pet| {
                    pet.image_columns()
                        .saturating_add(super::AMBIENT_PET_WRAP_GAP_COLUMNS)
                })
                .unwrap_or(0)
        }

        #[cfg(not(feature = "full-runtime-extensions"))]
        0
    }

    pub(crate) fn history_wrap_width(&self, width: u16) -> u16 {
        width
            .saturating_sub(self.ambient_pet_wrap_reserved_cols())
            .max(1)
    }
}
