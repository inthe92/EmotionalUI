use super::Context;
use crate::{editorial::{Profile, Signals, season_id}, library::Game, mood::Mood};
use std::collections::BTreeMap;

/// Integer point breakdown. The total is affinity, not a probability.
#[derive(Clone, Copy, Debug, Default)]
pub struct Score {
    pub context: i64,
    pub session: i64,
    pub preference: i64,
    pub novelty: i64,
    pub repetition: i64,
    pub variation: i64,
}
impl Score {
    pub fn total(self) -> i64 { self.context+self.session+self.preference+self.novelty-self.repetition+self.variation }
}
fn value(v: Option<u8>, divisor: i64) -> i64 { v.map_or(0,|v| i64::from(v)/divisor) }
fn certainty(p: &Profile) -> i64 {
    if p.knowledge.id.is_empty() { 100 } else { i64::from(p.knowledge.confidence) }
}
fn context(p: &Profile, ctx: Context, s: &Signals<'_>) -> i64 {
    let k = &p.knowledge;
    let mut n = 0;
    if s.date.is_some() {
        match ctx.mood {
            Mood::Dawn => { n += value(p.calm,10); if k.has("cheerful") || k.has("colourful") {n+=7;} if k.has("nature") {n+=5;} }
            Mood::Horizon => { n+=value(p.exploration,15)+value(p.intensity,25); }
            Mood::Shelter | Mood::Silence => {
                n+=value(p.mystery,10)+value(p.calm,10);
                if k.has("gothic") || k.has("dark_fantasy") { n+=10; }
                if k.has("science_fiction") {n+=5;}
                if ctx.mood==Mood::Silence { n+=value(p.calm,15); }
            }
        }
        match s.date.map(season_id) {
            Some("autumn") => { if k.has("gothic") || k.has("dark_fantasy") {n+=6;} n+=value(p.mystery,25); }
            Some("spring") => { if k.has("nature") {n+=8;} if k.has("colourful") {n+=5;} }
            Some("summer") => {if k.has("cheerful") {n+=5;} n+=value(p.intensity,30);}
            Some("winter") => { n+=value(p.calm,20)+value(p.nostalgia,25); }
            _ => {}
        }
        if crate::atlas::weekend(s.date) { n+=value(p.exploration,25); }
    }
    if ctx.raining { n+=value(p.calm,15)+value(p.mystery,25); }
    (n*certainty(p)/100).min(36)
}
pub fn score(g: &Game, ctx: Context, s: &Signals<'_>, personalized: bool, affinity: &BTreeMap<String,i8>) -> Score {
    let p=&g.editorial;
    let habits=s.habits.filter(|_|personalized);
    let target=habits.and_then(|h|h.target_minutes).unwrap_or(ctx.minutes);
    let mut session=0;
    // Never use library fallback 30 min or objective total completion time.
    if let Some(minutes)=p.session_minutes {
        session += if minutes<=target {12} else {-i64::from(minutes.saturating_sub(target)).min(12)};
    }
    if target<=15 {
        session += i64::from(*p.extra.get("short_sessions").unwrap_or(&0))/5;
        session += value(p.knowledge.interruptibility,5);
        session -= value(p.mental_demand,25);
    } else if target>=60 {
        session += value(p.exploration,6);
        if p.knowledge.has("story") {session+=8;}
    }
    session=session*certainty(p)/100;
    let mut preference=i64::from(g.favorite)*18+i64::from(g.started)*6;
    if personalized {
        preference+=i64::from(affinity.get(&g.id).copied().unwrap_or(0).clamp(-1,1))*100;
        if g.choices>=3 {preference+=i64::from(g.choices.min(10))*2;}
        if let Some(h)=habits {
            preference+=p.knowledge.genres.iter().filter_map(|t|h.genre_affinity.get(t)).max().copied().unwrap_or(0);
            preference+=h.context_choices.get(&g.id).copied().unwrap_or(0);
        }
    }
    let repetition=i64::from(g.recent_recommendations.min(3))*12
        +habits.and_then(|h|h.recent.get(&g.id)).copied().unwrap_or(0);
    // FNV with avalanche: bounded 0..3, reproducible for date slot + refresh + ID.
    let mut seed=g.id.bytes().fold(0xcbf29ce484222325u64,|h,b|(h^u64::from(b)).wrapping_mul(0x100000001b3));
    seed^=ctx.slot.wrapping_add(ctx.refresh.wrapping_mul(7919));
    seed=(seed^(seed>>30)).wrapping_mul(0xbf58476d1ce4e5b9);
    seed=(seed^(seed>>27)).wrapping_mul(0x94d049bb133111eb);
    Score { context:context(p,ctx,s),session,preference,
        novelty: if personalized {8-i64::from(g.choices.min(8))} else {8},
        repetition,variation:((seed^(seed>>31))%4) as i64 }
}
pub(super) fn similarity(a: &Game,b: &Game)->i64 {
    let ka=&a.editorial.knowledge; let kb=&b.editorial.knowledge;
    let genre=if !ka.genres.is_empty() && !kb.genres.is_empty() {
        ka.genres.iter().any(|x|kb.genres.contains(x))
    } else { a.genre!="Sin clasificar" && a.genre==b.genre };
    i64::from(genre)*12+i64::from(ka.settings.iter().any(|x|kb.settings.contains(x)))*4
        +i64::from(a.editorial.pace.zip(b.editorial.pace).is_some_and(|(x,y)|x.abs_diff(y)<25))*3
}
/// Reasons state supported attributes. No promise of save states or completion.
pub fn explain(g:&Game,ctx:Context,s:&Signals<'_>,personalized:bool,affinity:i8)->String {
    let p=&g.editorial; let k=&p.knowledge;
    let target=s.habits.filter(|_|personalized).and_then(|h|h.target_minutes).unwrap_or(ctx.minutes);
    if personalized && affinity>0 {return "Te interesa volver a darle un momento.".into();}
    if target<=15 && k.interruptibility.is_some_and(|v|v>=70) {
        return "Un puzle que puedes dejar a medias y retomar después.".into();
    }
    if target<=15 && p.extra.get("short_sessions").is_some_and(|v|*v>=60) {
        return "Objetivos separados para ese rato corto que tienes.".into();
    }
    if target>=60 && p.exploration.is_some_and(|v|v>=70) {return "Hoy tienes margen para explorar y seguir una ruta más larga.".into();}
    if s.date.is_some() && matches!(ctx.mood,Mood::Shelter|Mood::Silence) {
        if k.has("gothic") || k.has("dark_fantasy") {
            return if s.date.map(season_id)==Some("autumn") {"Monstruos y fantasía oscura para esta noche de otoño."} else {"Un mundo de fantasía oscura para adentrarte esta noche."}.into();
        }
        if p.mystery.is_some_and(|v|v>=60) {return "Esta noche invita a explorar y desentrañar sus misterios.".into();}
        if p.calm.is_some_and(|v|v>=60) {return "Una pausa más tranquila para cerrar el día a tu ritmo.".into();}
    }
    if s.date.map(season_id)==Some("spring") && k.has("nature") {return "Un cambio de aire entre naturaleza, aires de primavera y nuevas tareas.".into();}
    if s.date.is_some() && ctx.mood==Mood::Dawn && (k.has("colourful")||k.has("cheerful")) {return "Color y pequeños retos para empezar la mañana.".into();}
    if g.favorite {return "Uno de tus favoritos también tiene sitio en este momento.".into();}
    if p.exploration.is_some_and(|v|v>=70) {return "Rutas y rincones por descubrir, sin imponer un camino.".into();}
    if k.has("puzzle") {return "Un reto de ingenio para cambiar de ritmo.".into();}
    if k.has("racing") {return "Cambia de ritmo con una carrera de tu colección.".into();}
    if k.has("story") {return "Una historia de tu colección a la que dedicar un rato.".into();}
    if k.has("platforming") {return "Saltos y caminos alternativos para ponerte en marcha.".into();}
    if k.has("combat") {return "Un poco de acción para cambiar de aire.".into();}
    match g.id.bytes().map(u64::from).sum::<u64>()%3 {
        0=>"Una ocasión para descubrir este rincón de tu colección.",
        1=>"Lo tienes a mano; su ambiente aún está por descubrir.",
        _=>"Un juego de tu colección al que todavía podemos conocer mejor.",
    }.into()
}
