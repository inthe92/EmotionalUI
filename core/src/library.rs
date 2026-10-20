use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Emotions {
    pub calm: u8,
    pub intensity: u8,
    pub nostalgia: u8,
    pub mystery: u8,
    pub exploration: u8,
    pub tension: u8,
    pub joy: u8,
    pub concentration: u8,
    pub immersion: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub genre: String,
    pub available: bool,
    pub started: bool,
    pub favorite: bool,
    pub session_minutes: u16,
    pub choices: u32,
    pub recent_recommendations: u32,
    pub emotions: Emotions,
    #[serde(default)]
    pub editorial: crate::editorial::Profile,
    #[serde(default)]
    pub choices_by_mood: [u16; 4],
}
pub fn demo_catalog() -> Vec<Game> {
    [
        (
            "El faro de las mareas",
            "Vuelve donde lo dejaste. El mar aún te espera.",
            "Exploración",
            true,
            25,
            80,
            25,
        ),
        (
            "Jardín de nubes",
            "Cultiva un rincón de calma, a tu ritmo.",
            "Puzles",
            false,
            15,
            95,
            10,
        ),
        (
            "Órbita de papel",
            "Una pequeña nave. Un universo por descubrir.",
            "Aventura",
            false,
            30,
            50,
            55,
        ),
        (
            "Neón de bolsillo",
            "Una vuelta más bajo las luces de la ciudad.",
            "Carreras",
            false,
            10,
            15,
            95,
        ),
        (
            "El último tranvía",
            "Cada parada guarda una historia.",
            "Misterio",
            false,
            20,
            65,
            35,
        ),
        (
            "Bosque de cobre",
            "Pierde el camino. Encuentra una historia.",
            "Aventura",
            false,
            35,
            75,
            40,
        ),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(i, (title, subtitle, genre, started, minutes, calm, intensity))| Game {
            id: format!("demo-{i}"),
            title: title.into(),
            subtitle: subtitle.into(),
            genre: genre.into(),
            available: true,
            started,
            favorite: i == 0,
            session_minutes: minutes,
            choices: 0,
            editorial: crate::editorial::Profile::default(),
            choices_by_mood: [0; 4],
            recent_recommendations: 0,
            emotions: Emotions {
                calm,
                intensity,
                nostalgia: 60,
                mystery: 50,
                exploration: 70,
                tension: 20,
                joy: 65,
                concentration: 40,
                immersion: 75,
            },
        },
    )
    .collect()
}
