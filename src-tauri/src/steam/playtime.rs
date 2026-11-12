//! Temps de jeu Steam, lu dans `userdata/<accountid>/config/localconfig.vdf` (texte VDF). Lecture seule.
//!
//! Chemin dans le fichier : `UserLocalConfigStore > Software > Valve > Steam > apps > <appid> > Playtime`
//! (en minutes). La casse des clés varie selon les versions de Steam (`apps` / `Apps`) : on l'ignore.

use keyvalues_parser::{Obj, Parser, Value, Vdf};
use std::path::{Path, PathBuf};

pub fn localconfig_path(steam: &Path, account_id: u32) -> PathBuf {
    steam.join("userdata").join(account_id.to_string()).join("config").join("localconfig.vdf")
}

fn child<'a>(obj: &'a Obj<'a>, key: &str) -> Option<&'a Value<'a>> {
    obj.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).and_then(|(_, v)| v.first())
}

/// Minutes de jeu d'un jeu, d'après le texte de `localconfig.vdf`. `None` si absent ou illisible.
pub fn parse_playtime(text: &str, appid: u32) -> Option<u32> {
    // Steam double les barres obliques inverses ; à défaut, on retente en les prenant telles quelles
    let vdf: Vdf = Parser::new()
        .parse(text)
        .ok()
        .or_else(|| Parser::new().literal_special_chars(true).parse(text).ok())?
        .into();
    let mut node = vdf.value.get_obj()?;
    for key in ["Software", "Valve", "Steam", "apps"] {
        node = child(node, key)?.get_obj()?;
    }
    let app = child(node, &appid.to_string())?.get_obj()?;
    child(app, "Playtime")?.get_str()?.trim().parse().ok()
}

pub fn load(steam: &Path, account_id: u32, appid: u32) -> Option<u32> {
    let text = std::fs::read_to_string(localconfig_path(steam, account_id)).ok()?;
    parse_playtime(&text, appid)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
"UserLocalConfigStore"
{
	"friends"
	{
		"note"		"{\"json\":\"avec guillemets\"}"
	}
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"apps"
				{
					"620"
					{
						"LastPlayed"		"1700000000"
						"Playtime"		"1234"
						"Playtime2wks"		"56"
					}
					"105600"
					{
						"LastPlayed"		"1700000001"
					}
				}
			}
		}
	}
}
"#;

    #[test]
    fn temps_de_jeu() {
        assert_eq!(parse_playtime(SAMPLE, 620), Some(1234));
        assert_eq!(parse_playtime(SAMPLE, 105600), None, "jeu sans temps de jeu");
        assert_eq!(parse_playtime(SAMPLE, 1), None, "jeu inconnu");
        assert_eq!(parse_playtime(&SAMPLE.replace("\"apps\"", "\"Apps\""), 620), Some(1234), "casse ignorée");
        assert_eq!(parse_playtime("pas du VDF {", 620), None);
    }
}
