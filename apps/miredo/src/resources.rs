use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use eframe::egui::Color32;
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Palette {
    colors: HashMap<String, Color32>,
}

impl Palette {
    pub fn load(path: &Path, mode: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire la palette {}", path.display()))?;
        let root: Value = serde_json::from_str(&contents).context("palette.json invalide")?;
        let values = root
            .get(mode)
            .and_then(Value::as_object)
            .with_context(|| format!("La palette ne contient pas le thème « {mode} »"))?;

        let mut colors = HashMap::new();
        for (name, value) in values {
            let hex = value.as_str().with_context(|| {
                format!("La couleur « {name} » doit être une chaîne hexadécimale")
            })?;
            colors.insert(
                name.clone(),
                parse_color(hex)
                    .with_context(|| format!("Valeur invalide pour la couleur « {name} »"))?,
            );
        }

        for key in [
            "background",
            "surface",
            "surface_elevated",
            "surface_hover",
            "surface_selected",
            "border_subtle",
            "border_strong",
            "text_primary",
            "text_secondary",
            "text_muted",
            "accent",
            "accent_hover",
            "accent_active",
            "accent_text",
            "success",
            "warning",
            "danger",
            "focus",
            "viewer_background",
            "viewer_paper",
            "viewer_toolbar",
            "viewer_selection",
        ] {
            if !colors.contains_key(key) {
                bail!("La couleur « {key} » manque dans le thème « {mode} »");
            }
        }

        Ok(Self { colors })
    }

    pub fn get(&self, key: &str) -> Color32 {
        self.colors[key]
    }
}

fn parse_color(hex: &str) -> Result<Color32> {
    let raw = hex
        .strip_prefix('#')
        .context("la valeur doit commencer par #")?;
    if raw.len() != 6 {
        bail!("la valeur doit contenir six chiffres");
    }
    let red = u8::from_str_radix(&raw[0..2], 16)?;
    let green = u8::from_str_radix(&raw[2..4], 16)?;
    let blue = u8::from_str_radix(&raw[4..6], 16)?;
    Ok(Color32::from_rgb(red, green, blue))
}

#[derive(Clone, Debug)]
pub struct Translator {
    translations: HashMap<String, HashMap<String, String>>,
}

impl Translator {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Impossible de lire les traductions {}", path.display()))?;
        let translations: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&contents).context("i18n.json invalide")?;
        for locale in ["fr", "mg", "en"] {
            if !translations.contains_key(locale) {
                bail!("La langue « {locale} » manque dans i18n.json");
            }
        }
        Ok(Self { translations })
    }

    pub fn text(&self, locale: &str, key: &str) -> String {
        self.translations
            .get(locale)
            .and_then(|messages| messages.get(key))
            .or_else(|| {
                self.translations
                    .get("en")
                    .and_then(|messages| messages.get(key))
            })
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn palette_rejects_malformed_hex_values() {
        assert!(parse_color("#52786b").is_ok());
        assert!(parse_color("52786b").is_err());
        assert!(parse_color("#fff").is_err());
    }

    #[test]
    fn every_locale_has_the_same_translation_keys() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("i18n.json");
        let translator = Translator::load(&path).unwrap();
        let french: HashSet<_> = translator.translations["fr"].keys().collect();

        for locale in ["mg", "en"] {
            let keys: HashSet<_> = translator.translations[locale].keys().collect();
            assert_eq!(keys, french, "clés de traduction incohérentes pour {locale}");
        }
    }
}
