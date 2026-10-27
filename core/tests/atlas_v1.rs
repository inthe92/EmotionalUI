use chrono::{Datelike,NaiveDate};
use emotional_core::{atlas::{Habits,Observation},editorial::Signals,library::{Game,demo_catalog},mood::Mood,recommendation::{Context,rank_editorial,score,explain}};
use std::collections::BTreeMap;
fn context(date:&str,hour:u32,minutes:u16)->(Context,Signals<'static>) {
    let d=NaiveDate::parse_from_str(date,"%Y-%m-%d").unwrap();
    let at=d.and_hms_opt(hour,19,0).unwrap();
    (Context {mood:Mood::at(at),minutes,raining:false,slot:at.and_utc().timestamp() as u64/21600,refresh:0},Signals {date:Some(d),..Default::default()})
}
fn library()->Vec<Game> {
    let mut g=demo_catalog();g.truncate(5);
    for (i,v) in g.iter_mut().enumerate() {
        v.id=format!("fixture-{i}");v.started=false;v.favorite=false;v.editorial=Default::default();
        v.editorial.knowledge.confidence=100;v.editorial.knowledge.id=v.id.clone();
    }
    g[0].editorial.knowledge.settings=vec!["dark_fantasy".into()];g[0].editorial.mystery=Some(80);g[0].editorial.exploration=Some(80);
    g[1].editorial.calm=Some(90);g[1].editorial.knowledge.settings=vec!["nature".into()];g[1].editorial.knowledge.aesthetics=vec!["colourful".into()];
    g[2].editorial.knowledge.interruptibility=Some(100);g[2].editorial.extra.insert("short_sessions".into(),100);
    g[3].editorial.exploration=Some(100);g[3].editorial.knowledge.themes=vec!["story".into()];g[3].editorial.knowledge.genres=vec!["rpg".into()];
    g[4].editorial.intensity=Some(90);
    g
}
fn observations(g:&Game, hour:u8,seconds:u32,n:usize)->Vec<Observation> {
    let now=NaiveDate::from_ymd_opt(2026,10,24).unwrap().and_hms_opt(23,19,0).unwrap().and_utc().timestamp();
    (0..n).map(|i|Observation {game_id:g.id.clone(),started_utc:now-(i as i64+1)*86400,local_hour:hour,weekend:true,seconds,recommended:i%2==0}).collect()
}
#[test] fn a_autumn_night_is_affinity_not_title_or_date_playlist() {
    let g=library();let (c,s)=context("2026-10-24",23,30);
    let r=rank_editorial(&g,c,&s);
    assert!(r[..3].contains(&0));assert!(r[..3].contains(&1));
    assert!(score(&g[0],c,&s,true,&BTreeMap::new()).context>20);
    let mut renamed=g.clone();for v in &mut renamed {v.title="Unrelated title".into();}
    assert_eq!(r,rank_editorial(&renamed,c,&s));
    let (_,summer)=context("2026-07-25",23,30);
    assert!(rank_editorial(&g,c,&summer).contains(&0));
}
#[test] fn b_spring_morning_changes_affinity_without_exclusion() {
    let g=library();let (c,s)=context("2026-04-14",10,30);
    let r=rank_editorial(&g,c,&s);assert_eq!(r[0],1);assert_eq!(r.len(),5);
    assert!(explain(&g[1],c,&s,false,0).contains("primavera"));
}
#[test] fn c_short_sessions_learn_only_after_enough_reliable_samples() {
    let g=library();let (c,mut s)=context("2026-10-24",23,30);
    let now=s.date.unwrap().and_hms_opt(23,19,0).unwrap().and_utc().timestamp();
    let rows=observations(&g[0],23,12*60,7);
    assert_eq!(Habits::learn(&g,&rows[..2],Some((now,23,true))).target_minutes,None);
    let h=Habits::learn(&g,&rows,Some((now,23,true)));assert_eq!(h.target_minutes,Some(12));
    s.habits=Some(&h);assert_eq!(rank_editorial(&g,c,&s)[0],2);
    assert!(score(&g[2],c,&s,true,&BTreeMap::new()).session>25);
}
#[test] fn d_long_saturday_sessions_allow_deeper_exploration() {
    let g=library();let (c,mut s)=context("2026-10-24",16,30);
    let now=NaiveDate::from_ymd_opt(2026,10,24).unwrap().and_hms_opt(23,19,0).unwrap().and_utc().timestamp();
    let rows=observations(&g[0],16,120*60,6);
    let h=Habits::learn(&g,&rows,Some((now,16,true)));assert_eq!(h.target_minutes,Some(120));
    s.habits=Some(&h);assert_eq!(rank_editorial(&g,c,&s)[0],3);
    assert_eq!(s.date.unwrap().weekday().number_from_monday(),6);
}
#[test] fn e_new_user_gets_three_distinct_installed_candidates() {
    let g=library();let (c,s)=context("2026-10-24",23,30);
    let r=rank_editorial(&g,c,&s);assert_eq!(r.len(),5);assert_ne!(r[0],r[1]);assert_ne!(r[1],r[2]);
    assert_eq!(r,rank_editorial(&g,c,&s));
}
#[test] fn f_small_empty_missing_and_duplicate_library() {
    let mut g=library();let (c,s)=context("2026-10-24",23,30);
    assert_eq!(rank_editorial(&g,c,&s).len(),5);g.truncate(2);g[1].available=false;
    g.push(g[0].clone());assert_eq!(rank_editorial(&g,c,&s),vec![0]);
    assert!(rank_editorial(&[],c,&s).is_empty());
}
#[test] fn g_offline_and_unknown_clock_need_no_external_data() {
    let g=library();let (c,_)=context("2026-10-24",23,30);let s=Signals::default();
    assert_eq!(rank_editorial(&g,c,&s).len(),5);
    assert_eq!(score(&g[0],c,&s,false,&BTreeMap::new()).context,0);
    assert!(Habits::learn(&g,&[],None).target_minutes.is_none());
}
#[test] fn h_unknown_metadata_stays_unknown_and_no_fake_session_minutes() {
    let mut g=library();g[0].editorial=Default::default();let (c,s)=context("2026-10-24",23,10);
    assert_eq!(score(&g[0],c,&s,false,&BTreeMap::new()).session,0);
    assert_eq!(g[0].editorial.session_minutes,None);
    assert!(!explain(&g[0],c,&s,false,0).contains("oscura"));
}
#[test] fn preferences_diversity_refresh_and_private_mode() {
    let mut g=library();let (c,s)=context("2026-10-24",23,30);
    let affinity=BTreeMap::from([(g[4].id.clone(),1)]);
    assert!(score(&g[4],c,&s,true,&affinity).total()>100);
    let r=rank_editorial(&g,c,&s);let old=r[0];
    emotional_core::recommendation::renew(&mut g,&r);
    assert_ne!(rank_editorial(&g,Context {refresh:1,..c},&s)[0],old);
    let h=Habits {target_minutes:Some(10),..Default::default()};let mut personal=s.clone();personal.habits=Some(&h);
    assert_eq!(score(&g[2],c,&personal,false,&affinity).total(),score(&g[2],c,&s,false,&BTreeMap::new()).total());
}
#[test] fn invalid_future_stale_unknown_durations_and_weekday_samples_do_not_invent_habits() {
    let g=library();let mut rows=observations(&g[0],23,600,7);
    let now=rows[0].started_utc+86400;
    for r in &mut rows {r.started_utc=now+1;}
    assert_eq!(Habits::learn(&g,&rows,Some((now,23,true))).samples,0);
    for r in &mut rows {r.started_utc=now-181*86400;}
    assert_eq!(Habits::learn(&g,&rows,Some((now,23,true))).samples,0);
}
