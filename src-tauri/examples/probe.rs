//! Outil de diagnostic des sources Steam, sans interface.
//! Usage : cargo run --example probe -- <appid> [langue] [lien de profil]

use chasse_succes_lib::steam;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(appid) = args.first().and_then(|a| a.parse::<u32>().ok()) else {
        let d = steam::detect_all();
        println!("Steam : {:?}", d.steam_path);
        for a in &d.accounts {
            println!("Compte {} ({}) {:?} récent={}", a.account_id, a.steamid64, a.persona_name, a.most_recent);
        }
        println!("{} jeux en cache local", d.cached_games.len());
        eprintln!("usage : probe <appid> [langue] [profil]");
        return;
    };
    let lang = args.get(1).map(String::as_str).unwrap_or("french");
    let profile = args.get(2).map(String::as_str);

    let report = tauri::async_runtime::block_on(steam::probe(appid, lang, profile));
    for (label, outcome) in [
        ("CACHE LOCAL", &report.local_cache),
        ("PROFIL PUBLIC", &report.public_profile),
        ("PAGE PUBLIQUE", &report.public_page),
    ] {
        println!("\n=== {label} ===");
        match outcome {
            steam::Outcome::Err(e) => println!("ÉCHEC : {e}"),
            steam::Outcome::Ok(g) => {
                let n = g.achievements.len();
                let done = g.achievements.iter().filter(|a| a.unlocked == Some(true)).count();
                let ids = g.achievements.iter().filter(|a| a.api_name.is_some()).count();
                let hidden = g.achievements.iter().filter(|a| a.hidden).count();
                let hidden_desc = g.achievements.iter().filter(|a| a.hidden && !a.description.is_empty()).count();
                println!(
                    "{:?} — {n} succès, {done} obtenus, {ids} avec ID technique, {hidden} cachés ({hidden_desc} avec description), langue {}",
                    g.game_name, g.language
                );
                for a in g.achievements.iter().take(4) {
                    println!(
                        "  [{}] {:<28} {:<40} {:>6} {}",
                        match a.unlocked { Some(true) => "x", Some(false) => " ", None => "?" },
                        a.api_name.as_deref().unwrap_or("-"),
                        a.name,
                        a.rarity.map(|r| format!("{r:.1}%")).unwrap_or_default(),
                        a.unlock_time.map(|t| t.to_string()).unwrap_or_default(),
                    );
                }
            }
        }
    }
    println!("\nAPI rareté : {:?}", report.rarity_api_count);
    println!("Temps de jeu (minutes) : {:?}", steam::playtime_minutes(appid));
}
