use serde::{Deserialize, Serialize};
use actix_web::{
    middleware, rt,
    web::{self, Data, Form, Path},
    App, HttpRequest, HttpResponse, HttpServer,
};
use librespot::core::{keymaster, session::Session, spotify_id::SpotifyId};
use librespot::playback::player::PlayerEvent;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{sync::mpsc::SyncSender, thread};

use crate::db::{SpotifyDatabase, SpotifyTrack};
use crate::config::SpotifmConfig;
use crate::announce::{espeak, get_elevenlabs_tts, play_elevenlabs};

const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const SCOPES: &str =
    "streaming,user-read-playback-state,user-modify-playback-state,user-read-currently-playing";

#[derive(Serialize, Deserialize)]
pub struct AnnounceBumper {
    pub enable: Option<bool>,
    pub tag: Option<String>,
    pub freq: Option<usize>,
    pub speed: Option<u32>,
    pub amplitude: Option<u32>,
    pub voice: Option<String>,
    pub pitch: Option<u32>,
    pub gap: Option<u32>,
}

#[derive(Serialize, Deserialize)]
pub struct AnnounceSong {
    pub enable: Option<bool>,
    pub speed: Option<u32>,
    pub amplitude: Option<u32>,
    pub voice: Option<String>,
    pub pitch: Option<u32>,
    pub gap: Option<u32>,
}

#[get("/elevenlabs")]
pub async fn do_elevenlabs_say(
    req: HttpRequest,
    config: Data<Arc<Mutex<SpotifmConfig>>>,
) -> HttpResponse {
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();

    let text = query.get("text").unwrap().clone();

    get_elevenlabs_tts(text.as_str(), config.lock().unwrap().elevenlabs.clone());
    play_elevenlabs();

    return HttpResponse::Ok().json(HashMap::from([("text", text)]));
}

#[get("/espeak")]
pub async fn do_espeak_say(
    req: HttpRequest,
    config: Data<Arc<Mutex<SpotifmConfig>>>,
) -> HttpResponse {
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    espeak(query.get("text").unwrap().clone(), config.lock().unwrap().announce.song.espeak.clone());
    return HttpResponse::Ok().json(HashMap::from([("text", query.get("text").unwrap())]));
}

#[delete("/announce/bumper/tags")]
pub async fn delete_announce_bumper_tags(
    config: Data<Arc<Mutex<SpotifmConfig>>>,
) -> HttpResponse {
    config.lock().unwrap().announce.bumper.clear_tags();
    return HttpResponse::Ok().json(config.lock().unwrap().announce.bumper.clone());
}

#[post("/announce/song")]
pub async fn edit_announce_song(
    form: Form<AnnounceSong>,
    config: Data<Arc<Mutex<SpotifmConfig>>>,
) -> HttpResponse {
    if form.enable.is_some() {
        config.lock().unwrap().announce.song.enable = form.enable.unwrap();
    }

    if form.speed.is_some() {
        config.lock().unwrap().announce.song.espeak.speed = form.speed.unwrap();
    }

    if form.amplitude.is_some() {
        config.lock().unwrap().announce.song.espeak.amplitude = form.amplitude.unwrap();
    }

    if form.voice.is_some() {
        config.lock().unwrap().announce.song.espeak.voice = form.voice.clone().unwrap();
    }

    if form.pitch.is_some() {
        config.lock().unwrap().announce.song.espeak.pitch = form.pitch.unwrap();
    }

    if form.gap.is_some() {
        config.lock().unwrap().announce.song.espeak.gap = form.gap.unwrap();
    }    

    return HttpResponse::Ok().json(config.lock().unwrap().announce.song.clone());
}

#[post("/announce/bumper")]
pub async fn edit_announce_bumper(
    form: Form<AnnounceBumper>,
    config: Data<Arc<Mutex<SpotifmConfig>>>,
) -> HttpResponse {
    if form.enable.is_some() {
        config.lock().unwrap().announce.bumper.enable = form.enable.unwrap();
    }

    if form.tag.is_some() {
        config.lock().unwrap().announce.bumper.add_tag(form.tag.clone().unwrap());
    }

    if form.freq.is_some() {
        config.lock().unwrap().announce.bumper.freq = form.freq.unwrap();
    }

    if form.speed.is_some() {
        config.lock().unwrap().announce.bumper.espeak.speed = form.speed.unwrap();
    }

    if form.amplitude.is_some() {
        config.lock().unwrap().announce.bumper.espeak.amplitude = form.amplitude.unwrap();
    }

    if form.voice.is_some() {
        config.lock().unwrap().announce.bumper.espeak.voice = form.voice.clone().unwrap();
    }

    if form.pitch.is_some() {
        config.lock().unwrap().announce.bumper.espeak.pitch = form.pitch.unwrap();
    }

    if form.gap.is_some() {
        config.lock().unwrap().announce.bumper.espeak.gap = form.gap.unwrap();
    }    

    return HttpResponse::Ok().json(config.lock().unwrap().announce.bumper.clone());
}

#[get("/announce/{type}")]
pub async fn get_announce(
    path: Path<String>,
    config: Data<Arc<Mutex<SpotifmConfig>>>,
) -> HttpResponse {
    match path.0.as_str() {
        "bumper" => return HttpResponse::Ok().json(config.lock().unwrap().clone().announce.bumper),
        "song" => return HttpResponse::Ok().json(config.lock().unwrap().clone().announce.song),
        _ => return HttpResponse::NotFound().finish(),
    }
}

#[get("/search/{type}/{num}")]
pub async fn search(
    req: HttpRequest,
    path: Path<(String, u32)>,
    session: Data<Arc<Mutex<Session>>>,
) -> HttpResponse {
    let query = web::Query::<HashMap<String, String>>::from_query(req.query_string()).unwrap();
    return match spotify_get(
        session,
        format!(
            "https://api.spotify.com/v1/search?q={}&type={}&limit={}",
            query.get("q").unwrap(),
            path.0 .0.to_string().to_lowercase(),
            path.1
        ),
    )
    .await
    {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err)])),
        Ok(result) => HttpResponse::Ok().json(result),
    };
}

#[get("/np")]
pub async fn np(db: Data<SpotifyDatabase>) -> HttpResponse {
    return match db.current_track() {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err.unwrap().to_string())])),
        Ok(track) => HttpResponse::Ok().json(track),
    };
}

#[get("/prev")]
pub async fn prev_track(db: Data<SpotifyDatabase>) -> HttpResponse {
    return match db.prev_track() {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err.to_string())])),
        Ok(track) => HttpResponse::Ok().json(track),
    };
}

#[get("/next")]
pub async fn next_track(db: Data<SpotifyDatabase>) -> HttpResponse {
    return match db.next_track() {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err.to_string())])),
        Ok(track) => HttpResponse::Ok().json(track),
    };
}

#[get("/skip")]
pub async fn skip(data: Data<SyncSender<PlayerEvent>>, db: Data<SpotifyDatabase>) -> HttpResponse {
    let next_playing = db.next_track().unwrap();
    return match data.send(PlayerEvent::Stopped {
        play_request_id: 0,
        track_id: SpotifyId::from_base62("0").unwrap(),
    }) {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err.to_string())])),
        Ok(_) => {
            return HttpResponse::Ok().json(next_playing);
        }
    };
}

#[get("/shuffle")]
pub async fn shuffle(db: Data<SpotifyDatabase>) -> HttpResponse {
    return match db.shuffle() {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err.to_string())])),
        Ok(state) => HttpResponse::Ok().json(state.queue),
    };
}

#[get("/queue/{id}")]
pub async fn queue(
    path: Path<String>,
    data: Data<SyncSender<PlayerEvent>>,
    session: Data<Arc<Mutex<Session>>>,
    db: Data<SpotifyDatabase>,
) -> HttpResponse {
    let now_playing = db.current_track().unwrap();
    let next_playing = db.next_track().unwrap();

    if now_playing.id == path.0.as_str() {
        return HttpResponse::Ok().json(now_playing);
    } else if next_playing.id == path.0.as_str() {
        return HttpResponse::Ok().json(next_playing);
    }

    return match spotify_track(session, path.0.clone()).await {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err)])),
        Ok(spotify_track) => {
                    return match db.queue_track(spotify_track.clone()) {
                        Err(err) => {
                            HttpResponse::Ok().json(HashMap::from([("error", err.to_string())]))
                        }
                        Ok(_) => {
                            return match data.send(PlayerEvent::Changed {
                                old_track_id: now_playing.spotify_id(),
                                new_track_id: spotify_track.spotify_id(),
                            }) {
                                Err(err) => HttpResponse::Ok()
                                    .json(HashMap::from([("error", err.to_string())])),
                                Ok(_) => {
                                    return HttpResponse::Ok().json(spotify_track);
                                }
                            }
                        }
                    };
        }
    };
}

#[get("/play/{id}")]
pub async fn play(
    path: Path<String>,
    data: Data<SyncSender<PlayerEvent>>,
    session: Data<Arc<Mutex<Session>>>,
    db: Data<SpotifyDatabase>,
) -> HttpResponse {
    let now_playing = db.current_track().unwrap();
    let next_playing = db.next_track().unwrap();

    if now_playing.id == path.0.as_str() {
        return HttpResponse::Ok().json(now_playing);
    } else if next_playing.id == path.0.as_str() {
        return HttpResponse::Ok().json(next_playing);
    }

    return match spotify_track(session, path.0.clone()).await {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err)])),
        Ok(spotify_track) => {
                    return match db.queue_track(spotify_track.clone()) {
                        Err(err) => {
                            HttpResponse::Ok().json(HashMap::from([("error", err.to_string())]))
                        }
                        Ok(_) => {
                            return match data.send(PlayerEvent::Changed {
                                old_track_id: now_playing.spotify_id(),
                                new_track_id: spotify_track.spotify_id(),
                            }) {
                                Err(err) => HttpResponse::Ok()
                                    .json(HashMap::from([("error", err.to_string())])),
                                Ok(_) => {
                                    return match data.send(PlayerEvent::Stopped {
                                        play_request_id: 0,
                                        track_id: spotify_track.spotify_id(),
                                    }) {
                                        Err(err) => HttpResponse::Ok()
                                            .json(HashMap::from([("error", err.to_string())])),
                                        Ok(_) => HttpResponse::Ok().json(spotify_track),
                                    }
                                }
                            }
                        }
                    };
        }
    };
}

#[get("/playlist")]
pub async fn show_playlist(db: Data<SpotifyDatabase>) -> HttpResponse {
    return match db.read() {
        Err(err) => HttpResponse::Ok().json(HashMap::from([("error", err.to_string())])),
        Ok(state) => HttpResponse::Ok().json(state.queue),
    };
}

async fn spotify_get(
    session: Data<Arc<Mutex<Session>>>,
    url: String,
) -> Result<serde_json::Value, String> {
    return match keymaster::get_token(&session.lock().unwrap(), CLIENT_ID, SCOPES).await {
        Err(_) => Err("could not get token".to_string()),
        Ok(token) => match ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", token.access_token))
            .call()
        {
            Ok(response) => response
                .into_json::<serde_json::Value>()
                .map_err(|err| err.to_string()),
            Err(err) => Err(err.to_string()),
        },
    };
}

async fn spotify_track(
    session: Data<Arc<Mutex<Session>>>,
    track_id: String,
) -> Result<SpotifyTrack, String> {
    let track = spotify_get(
        session,
        format!("https://api.spotify.com/v1/tracks/{}", track_id),
    )
    .await?;

    let id = track
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or(track_id.as_str())
        .to_string();
    let name = track
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let artists = track
        .get("artists")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|artist| artist.get("name").and_then(|value| value.as_str()))
                .map(|name| name.to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Ok(SpotifyTrack::new(id, name, artists))
}

#[actix_rt::main]
pub async fn start(tx: SyncSender<PlayerEvent>, config: Arc<Mutex<SpotifmConfig>>, session: Arc<Mutex<Session>>, db: SpotifyDatabase) {
    thread::spawn(move || {
        //let session = session.lock().unwrap().clone();
        match rt::System::new("rest-api").block_on(
            HttpServer::new(move || {
                let tx = web::Data::new(tx.clone());
                let config = web::Data::new(config.clone());
                let session = web::Data::new(session.clone());
                let db = web::Data::new(db.clone());
                App::new()
                    .wrap(middleware::Logger::default())
                    .app_data(tx)
                    .app_data(config)
                    .app_data(session)
                    .app_data(db)
                    .service(np)
                    .service(prev_track)
                    .service(next_track)
                    .service(skip)
                    .service(queue)
                    .service(play)
                    .service(search)
                    .service(show_playlist)
                    .service(shuffle)
                    .service(get_announce)
                    .service(do_espeak_say)
                    .service(do_elevenlabs_say)
                    .service(edit_announce_song)
                    .service(edit_announce_bumper)
                    .service(delete_announce_bumper_tags)
            })
            .bind("0.0.0.0:9090")
            .unwrap()
            .run(),
        ) {
            Ok(_) => {}
            Err(err) => panic!("{}", err.to_string()),
        };
    });
}
