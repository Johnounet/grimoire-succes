use serde::Serialize;

/// D'où viennent les données d'un import.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    LocalCache,
    PublicProfile,
    PublicPage,
}

/// Un succès, quelle que soit sa source. Les champs absents d'une source restent à `None`.
#[derive(Debug, Clone, Serialize)]
pub struct Achievement {
    /// ID technique (ex. `ACH.WAKE_UP`), indispensable au re-lock.
    pub api_name: Option<String>,
    pub name: String,
    pub description: String,
    pub hidden: bool,
    pub icon_url: Option<String>,
    pub icon_gray_url: Option<String>,
    /// Rareté mondiale en %.
    pub rarity: Option<f32>,
    /// `None` = état inconnu pour cette source.
    pub unlocked: Option<bool>,
    /// Horodatage Unix de l'obtention.
    pub unlock_time: Option<i64>,
    /// AppID du DLC qui apporte ce succès, si connu.
    pub dlc_appid: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameAchievements {
    pub appid: u32,
    pub game_name: Option<String>,
    pub source: Source,
    /// Langue réellement obtenue (nom Steam : `french`, `english`…).
    pub language: String,
    pub achievements: Vec<Achievement>,
}

/// URL d'une icône de succès sur le CDN de Steam.
pub fn icon_url(appid: u32, file: &str) -> Option<String> {
    let file = file.trim();
    if file.is_empty() {
        return None;
    }
    if file.starts_with("http") {
        return Some(file.to_string());
    }
    Some(format!(
        "https://cdn.fastly.steamstatic.com/steamcommunity/public/images/apps/{appid}/{file}"
    ))
}
