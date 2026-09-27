use crate::{
    catalog::Catalog,
    errors::Result,
    model::{ChampionChange, ChangeGroup, ChangeVerdict, PatchChanges},
    web::Client,
};
use std::cmp::Ordering;

/// League site locale for each language of the app.
const SITE_LOCALES: [(&str, &str); 2] = [("es", "es-mx"), ("en", "en-us")];
const DEFAULT_SITE_LOCALE: &str = "en-us";
const CHAMPIONS_SECTION: &str = "id=\"patch-champions\"";
const SECTION: &str = "<h2";
const BLOCK: &str = "<div class=\"patch-change-block";
const PORTRAIT: &str = "/img/champion/";
const PORTRAIT_END: &str = ".png";
const CONTEXT: &str = "<blockquote class=\"blockquote context\">";
const CONTEXT_END: &str = "</blockquote>";
const GROUP: &str = "<h4";
const GROUP_END: &str = "</h4>";
const LINE: &str = "<li>";
const LINE_END: &str = "</li>";
const ARROW: char = '⇒';
const ENTITIES: [(&str, &str); 7] = [("&nbsp;", " "), ("&quot;", "\""), ("&#x27;", "'"), ("&#39;", "'"), ("&lt;", "<"), ("&gt;", ">"), ("&amp;", "&")];
/// Stats where a lower number helps the champion, in the languages of the notes.
const LOWER_IS_BETTER: [&str; 12] = [
    "enfriamiento",
    "reutilización",
    "cooldown",
    "costo",
    "coste",
    "cost",
    "tiempo de lanzamiento",
    "cast time",
    "tiempo de carga",
    "charge time",
    "daño recibido",
    "damage taken",
];
/// Words that turn those stats around, like a cooldown refund or reduction.
const HIGHER_IS_BETTER: [&str; 4] = ["reembolso", "refund", "reducción", "reduction"];
/// Numbers closer than this are the same.
const SAME_NUMBER: f64 = 1e-9;

/// The official patch notes page, in the app's language.
pub fn notes_url(patch: &str, language: &str) -> String {
    let locale = SITE_LOCALES.iter().find(|(code, _)| *code == language).map_or(DEFAULT_SITE_LOCALE, |(_, locale)| locale);
    format!("https://www.leagueoflegends.com/{locale}/news/game-updates/league-of-legends-patch-{}-notes/", patch.replace('.', "-"))
}

/// Champion changes of a patch, read from its official notes.
pub fn fetch(http: &Client, patch: &str, language: &str, catalog: &Catalog) -> Result<PatchChanges> {
    let notes_url = notes_url(patch, language);
    let html = http.get(&notes_url).send()?.error_for_status()?.text()?;
    Ok(PatchChanges { patch: patch.into(), champions: parse(&html, catalog), notes_url })
}

fn parse(html: &str, catalog: &Catalog) -> Vec<ChampionChange> {
    let Some(start) = html.find(CHAMPIONS_SECTION) else { return Vec::new() };
    let section = &html[start + CHAMPIONS_SECTION.len()..];
    let section = section.find(SECTION).map_or(section, |end| &section[..end]);
    section.split(BLOCK).skip(1).filter_map(|block| champion_change(block, catalog)).collect()
}

/// A champion's block; the portrait names the champion the same way in every language.
fn champion_change(block: &str, catalog: &Catalog) -> Option<ChampionChange> {
    let alias = between(block, PORTRAIT, PORTRAIT_END)?;
    let id = *catalog.champion_aliases.get(&alias.to_lowercase())?;
    let groups: Vec<ChangeGroup> = block.split(GROUP).enumerate().filter_map(|(i, part)| group(i, part)).collect();
    Some(ChampionChange {
        champion: catalog.champion(id),
        verdict: verdict(groups.iter().flat_map(|group| &group.lines)),
        context: between(block, CONTEXT, CONTEXT_END).map(text).unwrap_or_default(),
        groups,
        yours: false,
    })
}

/// An ability or the base stats with its changed lines; the part before the first title has none.
fn group(index: usize, part: &str) -> Option<ChangeGroup> {
    let (title, rest) = if index == 0 { ("", part) } else { part.split_once(GROUP_END)? };
    let title = title.split_once('>').map_or(title, |(_, title)| title);
    let lines: Vec<String> = rest.split(LINE).skip(1).map(|line| text(line.split(LINE_END).next().unwrap_or(line))).filter(|line| !line.is_empty()).collect();
    (!lines.is_empty()).then(|| ChangeGroup { title: text(title), lines })
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let rest = &text[text.find(start)? + start.len()..];
    Some(&rest[..rest.find(end)?])
}

/// The visible text of an HTML fragment.
fn text(html: &str) -> String {
    let mut plain = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => plain.push(c),
            _ => {}
        }
    }
    let plain = ENTITIES.iter().fold(plain, |plain, (entity, value)| plain.replace(entity, value));
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Buff when more lines make the champion stronger than weaker, nerf when fewer, adjusted when even.
fn verdict<'a>(lines: impl Iterator<Item = &'a String>) -> ChangeVerdict {
    let (stronger, weaker) = lines.filter_map(|line| stronger(line)).fold((0, 0), |(up, down), better| if better { (up + 1, down) } else { (up, down + 1) });
    match stronger.cmp(&weaker) {
        Ordering::Greater => ChangeVerdict::Buff,
        Ordering::Less => ChangeVerdict::Nerf,
        Ordering::Equal => ChangeVerdict::Adjusted,
    }
}

/// Whether a "stat: before ⇒ after" line makes the champion stronger; None when no number changes.
fn stronger(line: &str) -> Option<bool> {
    let (before, after) = line.split_once(ARROW)?;
    let (stat, before) = before.split_once(':').unwrap_or(("", before));
    let (old, new) = (numbers(before).sum::<f64>(), numbers(after).sum::<f64>());
    if (new - old).abs() < SAME_NUMBER {
        return None;
    }
    let stat = stat.to_lowercase();
    let lower_is_better = LOWER_IS_BETTER.iter().any(|word| stat.contains(word)) && !HIGHER_IS_BETTER.iter().any(|word| stat.contains(word));
    Some((new > old) != lower_is_better)
}

/// Every number in the text, like 16.5 or -1.
fn numbers(text: &str) -> impl Iterator<Item = f64> + '_ {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter_map(|word| word.trim_end_matches('.').parse::<f64>().ok().filter(|number| number.is_finite()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const NOTES: &str = r#"<h2 id="patch-champions">Campeones</h2>
        <div class="patch-change-block white-stone accent-before"><div><p><a href="x"><img src="https://ddragon.leagueoflegends.com/cdn/16.17.1/img/champion/Aatrox.png"></a></p>
        <h3 class="change-title" id="patch-aatrox"><a href="x">Aatrox</a></h3>
        <blockquote class="blockquote context"><p>Le daremos una mejora.</p></blockquote>
        <h4 class="change-detail-title ability-title"><img src="w.png">W: Cadenas Infernales</h4>
        <ul><li><strong>Enfriamiento</strong>: 20 / 18 seg ⇒ <strong>18 / 16.5 seg</strong></li></ul>
        <h4 class="change-detail-title ability-title"><img src="e.png">E: Impulso Siniestro</h4>
        <ul><li><strong>Tasa de Curación</strong>: 1.1% ⇒ <strong>1.3%</strong></li></ul></div></div>
        <div class="patch-change-block white-stone accent-before"><div><img src="https://x/img/champion/Unknown.png"><h4 class="t">Q</h4><ul><li>A: 1 ⇒ 2</li></ul></div></div>
        <h2 id="patch-items">Objetos</h2>
        <div class="patch-change-block"><img src="https://x/img/champion/Aatrox.png"><h4 class="t">Item</h4><ul><li>A: 1 ⇒ 2</li></ul></div>"#;

    #[test]
    fn reads_the_champions_of_the_notes() {
        let catalog = Catalog {
            champion_aliases: HashMap::from([("aatrox".into(), 266)]),
            champions: HashMap::from([(266, ("Aatrox".into(), "aatrox.png".into()))]),
            ..Catalog::default()
        };
        let changes = parse(NOTES, &catalog);
        assert_eq!(changes.len(), 1);
        let aatrox = &changes[0];
        assert_eq!((aatrox.champion.id, aatrox.verdict, aatrox.context.as_str()), (266, ChangeVerdict::Buff, "Le daremos una mejora."));
        assert_eq!(aatrox.groups[0].title, "W: Cadenas Infernales");
        assert_eq!(aatrox.groups[0].lines, ["Enfriamiento: 20 / 18 seg ⇒ 18 / 16.5 seg"]);
        assert_eq!(aatrox.groups.len(), 2);
    }

    #[test]
    fn tells_buffs_from_nerfs() {
        assert_eq!(stronger("Daño base: 70 / 110 ⇒ 80 / 120"), Some(true));
        assert_eq!(stronger("Enfriamiento: 140 / 115 seg ⇒ 160 / 130 seg"), Some(false));
        assert_eq!(stronger("Costo de Maná: 35 / 45 ⇒ 40 / 50"), Some(false));
        assert_eq!(stronger("Reembolso de enfriamiento al impacto: -1 seg ⇒ 1 seg"), Some(true));
        assert_eq!(stronger("Cooldown: 12 seconds ⇒ 10 seconds"), Some(true));
        assert_eq!(stronger("Daño: De 11 a 60 (niveles 1 a 18) ⇒ De 11 a 60 (niveles 1 a 18)"), None);
        assert_eq!(stronger("NUEVO: ahora también ralentiza"), None);
        let lines = |lines: &[&str]| lines.iter().map(|line| line.to_string()).collect::<Vec<_>>();
        let ryze = lines(&["Crecimiento de Armadura: 4.2 ⇒ 4.7", "Daño adicional: 25 / 50% ⇒ 15 / 40%", "Costo de Maná: 35 ⇒ 40"]);
        assert_eq!(verdict(ryze.iter()), ChangeVerdict::Nerf);
        let lucian = lines(&["Daño potenciado: 15 (+20% AD) ⇒ 5 (+15% AD)", "Daño base: 80 / 115 ⇒ 90 / 130"]);
        assert_eq!(verdict(lucian.iter()), ChangeVerdict::Adjusted);
    }

    #[test]
    fn links_the_notes_in_the_app_language() {
        assert_eq!(notes_url("26.19", "es"), "https://www.leagueoflegends.com/es-mx/news/game-updates/league-of-legends-patch-26-19-notes/");
        assert!(notes_url("26.19", "fr").contains("/en-us/"));
    }
}
