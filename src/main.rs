use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use governor::{DefaultDirectRateLimiter, Jitter, Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::time::Duration;
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeStruct;
use futures::{
    stream::futures_unordered::FuturesUnordered,
    StreamExt
};
use strum::IntoEnumIterator; // 0.17.1
use strum_macros::EnumIter; // 0.17.1

struct RateLimitedReqwestClient {
    reqwest_client: reqwest::Client,
    limiter: DefaultDirectRateLimiter,
    jitter: Jitter
}

impl RateLimitedReqwestClient {
    fn new() -> Self {
        Self {
            reqwest_client: reqwest::Client::new(),
            limiter: RateLimiter::direct(Quota::per_minute(nonzero!(300u32))),
            jitter: Jitter::up_to(Duration::from_secs(1)),
        }
    }

    async fn get<U>(&self, uri: U) -> reqwest::RequestBuilder where U: IntoUrl {
        self.limiter.until_ready_with_jitter(self.jitter).await;
        self.reqwest_client.get(uri)
    }
}

#[derive(Debug)]
enum OptionValue{
    Value(String),
    Table(HashMap<String, u32>),
}

impl OptionValue {
    fn insert(&mut self, value: String, weight: u32) -> Option<u32> {
        match self {
            OptionValue::Table(map) => {map.insert(value, weight)},
            _ => panic!("Can only insert into a table")
        }
    }
}

impl Serialize for OptionValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            OptionValue::Value(val) => {val.serialize(serializer)}
            OptionValue::Table(table) => {table.serialize(serializer)}
        }
    }
}

#[derive(Serialize, Debug)]
struct Trigger {
    option_category: String, // Always "Guild Wars 2"
    option_name: String,
    option_result: String,
    options: HashMap<String, HashMap<String, OptionValue>>,
}

impl Trigger {
    fn new(option_name: String, option_result: String) -> Self {
        Trigger {
            option_category: "Guild Wars 2".to_string(),
            option_name,
            option_result,
            options: HashMap::new(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Input {
    api_key: String,
    characters: HashMap<String, CharacterInput>,
}


const fn default_weight() -> u32 {50}
#[derive(Deserialize, Debug)]
struct CharacterInput {
    #[serde(default = "default_weight")]
    weight: u32,
    storyline: Option<HashMap<String, u32>>,
}

#[derive(Serialize, Debug)]
struct Output {
    name: String,
    description: String,
    game: String,
    #[serde(rename = "Guild Wars 2")]
    game_options: OutputOptions

}

impl Output {
    fn new() -> Self {
        Self {
            name: "Player{number}".to_string(),
            description: "Customized Guild Wars 2 Template".to_string(),
            game: "Guild Wars 2".to_string(),
            game_options: OutputOptions::new(),
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        let mut val = Self::new();
        val.game_options = OutputOptions::default();

        val
    }
}

#[derive(Serialize, Debug)]
struct OutputOptions {
    progression_balancing: HashMap<String, u32>,
    accessibility: HashMap<String, u32>,
    character: HashMap<String, u32>,
    triggers: Vec<Trigger>,
    character_profession: HashMap<String, u32>,
    character_race: HashMap<String, u32>,
    starting_mainhand_weapon: HashMap<String, u32>,
    starting_offhand_weapon: HashMap<String, u32>,
    group_content: HashMap<String, u32>,
    include_competitive: HashMap<String, u32>,
    achievement_weight: HashMap<String, u32>,
    quest_weight: HashMap<String, u32>,
    training_weight: HashMap<String, u32>,
    world_boss_weight: HashMap<String, u32>,
    storyline: HashMap<String, u32>,
    required_mist_fragments: u32,
    extra_mist_fragments: u32,
    heal_skill: HashMap<String, u32>,
    gear_slots: HashMap<String, u32>,
}

impl OutputOptions {
    fn new() -> Self {
        OutputOptions {
            progression_balancing: HashMap::new(),
            accessibility: HashMap::new(),
            character: HashMap::new(),
            triggers: Vec::new(),
            character_profession: HashMap::new(),
            character_race: HashMap::new(),
            starting_mainhand_weapon: HashMap::new(),
            starting_offhand_weapon: HashMap::new(),
            group_content: HashMap::new(),
            include_competitive: HashMap::new(),
            achievement_weight: HashMap::new(),
            quest_weight: HashMap::new(),
            training_weight: HashMap::new(),
            world_boss_weight: HashMap::new(),
            storyline: HashMap::new(),
            required_mist_fragments: 10,
            extra_mist_fragments: 5,
            heal_skill: HashMap::new(),
            gear_slots: HashMap::new(),
        }
    }
}

impl Default for OutputOptions {
    fn default() -> Self {
        let mut val = Self::new();

        val.progression_balancing.insert("random".to_string(), 0);
        val.progression_balancing.insert("random-low".to_string(), 0);
        val.progression_balancing.insert("random-high".to_string(), 0);
        val.progression_balancing.insert("disabled".to_string(), 0);
        val.progression_balancing.insert("normal".to_string(), 50);
        val.progression_balancing.insert("extreme".to_string(), 0);

        val.accessibility.insert("locations".to_string(), 0);
        val.accessibility.insert("items".to_string(), 50);
        val.accessibility.insert("minimal".to_string(), 0);

        val.starting_mainhand_weapon.insert("none".to_string(), 0);
        val.starting_mainhand_weapon.insert("axe".to_string(), 0);
        val.starting_mainhand_weapon.insert("dagger".to_string(), 0);
        val.starting_mainhand_weapon.insert("mace".to_string(), 0);
        val.starting_mainhand_weapon.insert("pistol".to_string(), 0);
        val.starting_mainhand_weapon.insert("sword".to_string(), 0);
        val.starting_mainhand_weapon.insert("scepter".to_string(), 0);
        val.starting_mainhand_weapon.insert("greatsword".to_string(), 0);
        val.starting_mainhand_weapon.insert("hammer".to_string(), 0);
        val.starting_mainhand_weapon.insert("longbow".to_string(), 0);
        val.starting_mainhand_weapon.insert("rifle".to_string(), 0);
        val.starting_mainhand_weapon.insert("short_bow".to_string(), 0);
        val.starting_mainhand_weapon.insert("staff".to_string(), 0);
        val.starting_mainhand_weapon.insert("random_proficient".to_string(), 50);
        val.starting_mainhand_weapon.insert("random_proficient_one_handed".to_string(), 0);
        val.starting_mainhand_weapon.insert("random_proficient_two_handed".to_string(), 0);

        val.starting_offhand_weapon.insert("none".to_string(), 0);
        val.starting_offhand_weapon.insert("scepter".to_string(), 0);
        val.starting_offhand_weapon.insert("focus".to_string(), 0);
        val.starting_offhand_weapon.insert("shield".to_string(), 0);
        val.starting_offhand_weapon.insert("torch".to_string(), 0);
        val.starting_offhand_weapon.insert("warhorn".to_string(), 0);
        val.starting_offhand_weapon.insert("random_proficient".to_string(), 50);

        val.group_content.insert("none".to_string(), 50);
        val.group_content.insert("five_man".to_string(), 25);
        val.group_content.insert("ten_man".to_string(), 10);

        val.include_competitive.insert("false".to_string(), 50);
        val.include_competitive.insert("true".to_string(), 10);

        val.achievement_weight.insert("500".to_string(), 50);
        val.achievement_weight.insert("random".to_string(), 0);
        val.achievement_weight.insert("random-low".to_string(), 0);
        val.achievement_weight.insert("random-high".to_string(), 0);

        val.quest_weight.insert("100".to_string(), 50);
        val.quest_weight.insert("random".to_string(), 0);
        val.quest_weight.insert("random-low".to_string(), 0);
        val.quest_weight.insert("random-high".to_string(), 0);

        val.training_weight.insert("100".to_string(), 50);
        val.training_weight.insert("random".to_string(), 0);
        val.training_weight.insert("random-low".to_string(), 0);
        val.training_weight.insert("random-high".to_string(), 0);

        val.world_boss_weight.insert("250".to_string(), 50);
        val.world_boss_weight.insert("random".to_string(), 0);
        val.world_boss_weight.insert("random-low".to_string(), 0);
        val.world_boss_weight.insert("random-high".to_string(), 0);

        val.heal_skill.insert("randomize".to_string(), 1);
        val.heal_skill.insert("early".to_string(), 10);
        val.heal_skill.insert("starting".to_string(), 50);

        val.gear_slots.insert("randomize".to_string(), 5);
        val.gear_slots.insert("early".to_string(), 50);
        val.gear_slots.insert("starting".to_string(), 10);

        val
    }
}

#[derive(Deserialize, Debug)]
struct Character {
    name: String,
    race: String,
    profession: String,
}

#[derive(EnumIter)]
enum Storyline {
    Core,
    Season1,
    Season2,
    HeartOfThorns,
    Season3,
    PathOfFire,
    Season4,
    IcebroodSaga,
    EndOfDragons,
    SecretsOfTheObscure,
}

impl Storyline {
    const fn id(&self) -> &str {
        match self {
            Storyline::Core => "215AAA0F-CDAC-4F93-86DA-C155A99B5784",
            Storyline::Season1 => "A49D0CD7-E725-4141-8E10-180F1CED7CAF",
            Storyline::Season2 => "A515A1D3-4BD7-4594-AE30-2C5D05FF5960",
            Storyline::HeartOfThorns => "B8901E58-DC9D-4525-ADB2-79C93593291E",
            Storyline::Season3 => "09766A86-D88D-4DF2-9385-259E9A8CA583",
            Storyline::PathOfFire => "EAB597C0-C484-4FD3-9430-31433BAC81B6",
            Storyline::Season4 => "C22AFD21-667A-4AA8-8210-AC74EAEE58BB",
            Storyline::IcebroodSaga => "EDCAE800-302A-4D9B-8331-3CC769ADA0B3",
            Storyline::EndOfDragons => "D1B709AB-92B6-4EE9-8B40-2B7C628E5022",
            Storyline::SecretsOfTheObscure => "AEE99452-D323-4ABB-8F49-D7C0A752CBD1",
        }
    }

    const fn snake_case(&self) -> &str {
        match self {
            Storyline::Core => "core",
            Storyline::Season1 => "season_1",
            Storyline::Season2 => "season_2",
            Storyline::HeartOfThorns => "heart_of_thorns",
            Storyline::Season3 => "season_3",
            Storyline::PathOfFire => "path_of_fire",
            Storyline::Season4 => "season_4",
            Storyline::IcebroodSaga => "icebrood_saga",
            Storyline::EndOfDragons => "end_of_dragons",
            Storyline::SecretsOfTheObscure => "secrets_of_the_obscure",
        }
    }

    const fn default_weight(&self) -> u32 {
        match self {
            Storyline::Core => 1,
            Storyline::Season1 => 2,
            Storyline::Season2 => 4,
            Storyline::HeartOfThorns => 8,
            Storyline::Season3 => 16,
            Storyline::PathOfFire => 32,
            Storyline::Season4 => 64,
            Storyline::IcebroodSaga => 128,
            Storyline::EndOfDragons => 256,
            Storyline::SecretsOfTheObscure => 512,
        }
    }

    const fn max_quests(&self) -> usize {
        match self {
            Storyline::Core => 49,
            Storyline::Season1 => 30,
            Storyline::Season2 => 32,
            Storyline::HeartOfThorns => 16,
            Storyline::Season3 => 36,
            Storyline::PathOfFire => 16,
            Storyline::Season4 => 30,
            Storyline::IcebroodSaga => 41,
            Storyline::EndOfDragons => 27,
            Storyline::SecretsOfTheObscure => 20,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Season {
    id: String,
    #[serde(rename = "stories")]
    story_ids: HashSet<u32>,
}

#[derive(Deserialize, Debug)]
struct Quest {
    id: u32,
    name: String,
    #[serde(rename = "story")]
    story_id: u32,
}

#[tokio::main]
async fn main() {
    let input: Input = {
        let file = fs::File::open("input.yaml").unwrap();
        let reader = BufReader::new(file);
        serde_yaml::from_reader(reader).unwrap()
    };
    println!("{:?}", input);

    let reqwest_client = Arc::new(RateLimitedReqwestClient::new());

    let character_names = {
        let uri = format!("https://api.guildwars2.com/v2/characters?access_token={}", input.api_key);
        let response = reqwest_client.get(&uri).await.send().await.unwrap();
        let mut characters = response.json::<HashSet<String>>().await.unwrap();
        if input.characters.len() > 0 {
            characters.drain().filter(|char| {input.characters.contains_key(char)}).collect()
        }
        else {
            characters
        }
    };

    println!("{:?}", character_names);

    let characters = {
        let mut tasks = FuturesUnordered::new();
        for name in &character_names {
            let uri =
                format!("https://api.guildwars2.com/v2/characters/{}/core?access_token={}",
                        name,
                        input.api_key);
            tasks.push(tokio::spawn(reqwest_client.get(uri).await.send()));
        }

        let mut characters = HashMap::new();
        while let Some(finished_task) = tasks.next().await {
            let character: Character = finished_task.unwrap().unwrap().json().await.unwrap();
            characters.insert(character.name.clone(), character);
        }

        characters
    };

    let seasons = {
        let mut tasks = FuturesUnordered::new();
        for storyline in Storyline::iter() {
            let uri = format!("https://api.guildwars2.com/v2/stories/seasons/{}",
                              storyline.id());
            println!("{}", uri);
            tasks.push(tokio::spawn(reqwest_client.get(uri).await.send()));
        }

        let mut seasons = HashMap::<String, Season>::new();
        while let Some(finished_task) = tasks.next().await {
            let season: Season = finished_task.unwrap().unwrap().json().await.unwrap();
            seasons.insert(season.id.clone(), season);
        }

        seasons
    };

    let quest_ids = {
        let response = reqwest_client.get("https://api.guildwars2.com/v2/quests").await.send().await.unwrap();
        response.json::<Vec<u32>>().await.unwrap()
    };

    let quests = {
        let mut quests = HashMap::new();

        let mut tasks = FuturesUnordered::new();
        for quest_chunk in quest_ids.as_slice().chunks(100) {
            let uri = quest_chunk.iter().fold("https://api.guildwars2.com/v2/quests?ids=".to_string(),
                                                 |str, id| format!("{}{},", str, id)
            );
            println!("{}", uri);
            tasks.push(tokio::spawn(reqwest_client.get(uri).await.send()));
            // categories.extend(reqwest_client.get(uri).await.send().await.unwrap().json::<Vec<AchievementCategory>>().await.unwrap());
        }

        while let Some(finished_task) = tasks.next().await {
            let mut element_vec = finished_task.unwrap().unwrap().json::<Vec<Quest>>().await.unwrap();
            let kv_iter = element_vec.drain(..).map(|q| (q.id, q));
            quests.extend(kv_iter);
        }

        quests
    };


    let mut output = Output::default();
    for (character_name, character_options) in input.characters {
        let character = characters.get(&character_name);

        let weight = character_options.weight;
        output.game_options.character.insert(character_name.clone(), weight);

        let mut trigger = Trigger::new("character".to_string(), character_name.clone());
        trigger.options.insert("Guild Wars 2".to_string(), HashMap::new());


        trigger.options.get_mut("Guild Wars 2").unwrap()
            .insert("character_profession".to_string(), OptionValue::Table(HashMap::new()));
        trigger.options.get_mut("Guild Wars 2").unwrap()
            .insert("character_race".to_string(), OptionValue::Table(HashMap::new()));

        let completed_quest_ids;
        let profession;
        let race;
        if let Some(character) = character {
            profession = character.profession.clone();
            race = character.race.clone();

            completed_quest_ids = Some({
                let uri = format!("https://api.guildwars2.com/v2/characters/{}/quests?access_token={}", &character_name, input.api_key);
                println!("{}", uri);
                let response = reqwest_client.get(uri).await.send().await.unwrap();
                response.json::<HashSet<u32>>().await.unwrap()
            });

        }
        else {
            profession = "random".to_string();
            race = "random".to_string();
            completed_quest_ids = None;
        }

        trigger.options.get_mut("Guild Wars 2").unwrap()
            .get_mut("character_profession").unwrap()
            .insert(profession, default_weight());
        trigger.options.get_mut("Guild Wars 2").unwrap()
            .get_mut("character_race").unwrap()
            .insert(race, default_weight());

        trigger.options.get_mut("Guild Wars 2").unwrap()
            .insert("storyline".to_string(), OptionValue::Table(HashMap::new()));

        let storyline_options = character_options.storyline;
        let mut storyline_triggers = Vec::new();
        for storyline in Storyline::iter() {

            let weight = if let Some (options) = &storyline_options {
                if options.contains_key(storyline.snake_case()) {
                    options[storyline.snake_case()]
                }
                else {
                    continue;
                }
            } else {
                storyline.default_weight()
            };

            let season = &seasons[storyline.id()];
            let completed_count =
                if let Some(completed) = &completed_quest_ids {
                     completed.iter().filter(|&q| season.story_ids.contains(&quests[q].story_id)).count()
                }
                else {
                    0
                }
            ;
            println!("{}", character_name);
            println!("{:?}, count: {}", completed_quest_ids, completed_count);
            println!("{}: {:?}", storyline.snake_case(), season);


            // for (id, quest) in quests.iter().filter(|(&id, q)| season.story_ids.contains(&q.story_id)) {
            //     println!("{}: {}", quest.name, if completed_quest_ids.contains(&id) {"Complete"} else {"Incomplete"});
            // }

            let intermediate_option_result = format!("{} {}", storyline.snake_case().to_string(), character_name.clone());
            trigger.options.get_mut("Guild Wars 2").unwrap()
                .get_mut("storyline").unwrap()
                .insert(intermediate_option_result.clone(), weight);

            let mut quest_trigger = Trigger::new("storyline".to_string(), intermediate_option_result);
            quest_trigger.options.insert("Guild Wars 2".to_string(), HashMap::new());
            quest_trigger.options.get_mut("Guild Wars 2").unwrap()
                .insert("max_quests".to_string(), OptionValue::Value(format!("{}", storyline.max_quests() - completed_count)));
            quest_trigger.options.get_mut("Guild Wars 2").unwrap()
                .insert("storyline".to_string(), OptionValue::Value(storyline.snake_case().to_string()));

            storyline_triggers.push(quest_trigger);
        }

        output.game_options.triggers.push(trigger);
        output.game_options.triggers.extend(storyline_triggers)
    }

    let file = File::create("gw2.yaml").unwrap();
    serde_yaml::to_writer(file, &output).unwrap();

}
