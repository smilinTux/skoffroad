use bevy::prelude::*;
use rand::prelude::*;
use std::time::Duration;
use super::cb_radio::{CBRadio, CBRadioManager, calculate_signal_strength};
use std::collections::HashMap;
use std::collections::VecDeque;

/// Component for AI truckers that can transmit on CB
#[derive(Component)]
pub struct AITrucker {
    handle: String,
    location: String,
    next_transmission: Timer,
    response_chance: f32,
    // New fields for enhanced behavior
    route: Vec<String>,           // Sequence of locations the trucker follows
    route_index: usize,          // Current position in route
    speed: f32,                  // Current travel speed
    mood: TruckerMood,          // Current mood affecting message style
    active_conversation: bool,   // Whether participating in ongoing chat
    last_heard_message: Option<String>, // Last message heard for context
    /// Preferred channels for this trucker
    preferred_channels: Vec<u8>,
    /// Current conversation state
    conversation_state: TruckerConversationState,
    /// Knowledge of road conditions
    road_knowledge: HashMap<String, RoadCondition>,
    /// Personality traits affecting communication
    personality: TruckerPersonality,
}

/// Trucker mood affecting message style
#[derive(Clone, Copy, Debug)]
pub enum TruckerMood {
    Friendly,
    Cautious,
    Helpful,
    Tired,
    Chatty,
}

impl Default for TruckerMood {
    fn default() -> Self {
        Self::Friendly
    }
}

impl Default for AITrucker {
    fn default() -> Self {
        Self {
            handle: String::new(),
            location: String::new(),
            next_transmission: Timer::from_seconds(30.0, TimerMode::Once),
            response_chance: 0.3,
            route: Vec::new(),
            route_index: 0,
            speed: 60.0, // mph
            mood: TruckerMood::default(),
            active_conversation: false,
            last_heard_message: None,
            preferred_channels: Vec::new(),
            conversation_state: TruckerConversationState::Listening,
            road_knowledge: HashMap::new(),
            personality: TruckerPersonality::default(),
        }
    }
}

/// Resource for managing AI chatter generation
#[derive(Resource)]
pub struct ChatterConfig {
    /// Minimum time between any transmissions
    pub min_transmission_interval: f32,
    /// Maximum number of AI truckers
    pub max_truckers: usize,
    /// Base chance for a response to another transmission
    pub base_response_chance: f32,
    /// List of trucker handles to choose from
    pub handles: Vec<String>,
    /// List of locations to reference
    pub locations: Vec<String>,
    /// Common routes between locations
    pub routes: Vec<Vec<String>>,
    /// Terrain interference factors
    pub terrain_interference: TerrainInterference,
}

/// Configuration for terrain-based interference
#[derive(Clone, Debug)]
pub struct TerrainInterference {
    pub mountain_factor: f32,    // Signal reduction in mountainous areas
    pub urban_factor: f32,       // Signal reduction in urban areas
    pub tunnel_factor: f32,      // Signal reduction in tunnels
    pub weather_factor: f32,     // Current weather impact on signals
}

impl Default for TerrainInterference {
    fn default() -> Self {
        Self {
            mountain_factor: 0.7,
            urban_factor: 0.5,
            tunnel_factor: 0.9,
            weather_factor: 1.0,
        }
    }
}

impl Default for ChatterConfig {
    fn default() -> Self {
        Self {
            min_transmission_interval: 5.0,
            max_truckers: 10,
            base_response_chance: 0.3,
            handles: vec![
                "Big Rig".into(), "Night Rider".into(), "Road King".into(),
                "Steel Horse".into(), "Mountain Man".into(), "Bear Bait".into(),
                "Chrome Dome".into(), "Gear Jammer".into(), "Rubber Duck".into(),
            ],
            locations: vec![
                "Mile Marker 145".into(), "Flying J".into(), "Love's Truck Stop".into(),
                "Rest Area".into(), "Weigh Station".into(), "Toll Plaza".into(),
                "Exit 23".into(), "County Line".into(), "State Line".into(),
            ],
            routes: vec![
                // Example routes
                vec!["Flying J".into(), "Mile Marker 145".into(), "Rest Area".into()],
                vec!["Love's Truck Stop".into(), "Weigh Station".into(), "Exit 23".into()],
                vec!["Toll Plaza".into(), "County Line".into(), "State Line".into()],
            ],
            terrain_interference: TerrainInterference::default(),
        }
    }
}

/// Plugin for CB radio AI chatter
pub struct CBChatterPlugin;

impl Plugin for CBChatterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatterConfig>()
           .init_resource::<WorldState>()
           .init_resource::<WeatherSystem>()
           .init_resource::<EmergencyServicesAI>()  // Add this line
           .add_systems(Startup, setup_ai_truckers)
           .add_systems(Update, (
               update_world_state,
               update_ai_transmissions,
               handle_responses,
               manage_conversation_flow,
               share_road_knowledge,
               handle_weather_reports,
               handle_emergencies,
               manage_event_chains,
               EventChainManager::update_emergency_chains,
               update_emergency_services,  // Add this line
           ).chain());
    }
}

/// Message types that AI truckers can generate
#[derive(Clone, Copy, Debug)]
enum MessageType {
    TrafficReport,
    WeatherWarning,
    BearReport,
    Location,
    General,
    Response,
    Conversation,  // New type for ongoing conversations
    Emergency,     // New type for urgent messages
}

impl MessageType {
    fn generate(&self, trucker: &AITrucker, topic: &ConversationTopic) -> String {
        let mut rng = rand::thread_rng();
        
        // Get personality-influenced message style
        let style = if trucker.personality.professionalism > 0.7 {
            "professional"
        } else if trucker.personality.humor > 0.7 {
            "humorous"
        } else {
            "casual"
        };
        
        match (self, topic) {
            (MessageType::TrafficReport, ConversationTopic::Traffic) => {
                let conditions = match style {
                    "professional" => [
                        "experiencing heavy congestion",
                        "moving at reduced speed",
                        "showing significant delays",
                    ],
                    "humorous" => [
                        "packed tighter than a truck stop diner",
                        "moving slower than my mother-in-law",
                        "backed up like a bad fuel pump",
                    ],
                    _ => [
                        "pretty backed up",
                        "slow going",
                        "stop and go",
                    ],
                };
                format!("{} here. Traffic's {} around {}.",
                    trucker.handle,
                    conditions[rng.gen_range(0..conditions.len())],
                    trucker.location
                )
            },
            
            (MessageType::WeatherWarning, ConversationTopic::Weather) => {
                let conditions = match style {
                    "professional" => [
                        "reduced visibility due to fog",
                        "hazardous road conditions from ice",
                        "strong crosswinds affecting stability",
                    ],
                    "humorous" => [
                        "fog thicker than pea soup",
                        "roads slicker than a greased pig",
                        "wind's blowin' like my dispatcher on Monday",
                    ],
                    _ => [
                        "heavy fog ahead",
                        "real icy patches",
                        "strong winds",
                    ],
                };
                format!("Heads up from {}. Got {} at {}. Watch yourself.",
                    trucker.handle,
                    conditions[rng.gen_range(0..conditions.len())],
                    trucker.location
                )
            },
            
            (MessageType::Maintenance, ConversationTopic::Maintenance) => {
                let issues = match style {
                    "professional" => [
                        "air pressure system requires inspection",
                        "brake components showing wear",
                        "transmission fluid needs checking",
                    ],
                    "humorous" => [
                        "got more rattles than a baby store",
                        "brakes squealing like a pig at feeding time",
                        "transmission's grumpier than a bear in winter",
                    ],
                    _ => [
                        "air pressure's acting up",
                        "brakes need looking at",
                        "transmission's giving trouble",
                    ],
                };
                format!("{} here. Anyone else {}? Looking for a good shop nearby.",
                    trucker.handle,
                    issues[rng.gen_range(0..issues.len())]
                )
            },
            
            (MessageType::FuelPrices, ConversationTopic::FuelPrices) => {
                let prices = [3.99, 4.15, 4.29, 4.49];
                let price = prices[rng.gen_range(0..prices.len())];
                format!("{} with a fuel update. {}'s showing ${:.2} a gallon{}",
                    trucker.handle,
                    trucker.location,
                    price,
                    if trucker.personality.helpfulness > 0.7 {
                        ". They're taking cards and cash, no wait at the pumps."
                    } else {
                        "."
                    }
                )
            },
            
            (MessageType::LoadInfo, ConversationTopic::LoadInfo) => {
                let loads = match style {
                    "professional" => [
                        "oversized load requiring escort",
                        "refrigerated goods time-sensitive",
                        "hazmat transport following route restrictions",
                    ],
                    "humorous" => [
                        "load's big enough to have its own ZIP code",
                        "carrying enough ice cream to feed a small army",
                        "got more paperwork than a tax office",
                    ],
                    _ => [
                        "big load coming through",
                        "reefer load running late",
                        "hazmat load on board",
                    ],
                };
                format!("{} here. {}. Any tips for the route ahead?",
                    trucker.handle,
                    loads[rng.gen_range(0..loads.len())]
                )
            },
            
            _ => format!("{} checking in from {}.",
                trucker.handle,
                trucker.location
            ),
        }
    }
}

/// System to set up initial AI truckers
fn setup_ai_truckers(
    mut commands: Commands,
    config: Res<ChatterConfig>,
) {
    let mut rng = rand::thread_rng();
    
    for _ in 0..config.max_truckers {
        let handle = config.handles.choose(&mut rng).unwrap().clone();
        let route = config.routes.choose(&mut rng).unwrap().clone();
        let mood = match rng.gen_range(0..5) {
            0 => TruckerMood::Friendly,
            1 => TruckerMood::Cautious,
            2 => TruckerMood::Helpful,
            3 => TruckerMood::Tired,
            _ => TruckerMood::Chatty,
        };
        
        commands.spawn(AITrucker {
            handle,
            location: route[0].clone(),
            next_transmission: Timer::from_seconds(
                rng.gen_range(5.0..30.0),
                TimerMode::Once
            ),
            response_chance: config.base_response_chance,
            route,
            route_index: 0,
            speed: rng.gen_range(55.0..75.0),
            mood,
            active_conversation: false,
            last_heard_message: None,
            preferred_channels: Vec::new(),
            conversation_state: TruckerConversationState::Listening,
            road_knowledge: HashMap::new(),
            personality: TruckerPersonality::default(),
        });
    }
}

/// Calculate interference based on terrain and conditions
fn calculate_interference(
    position: Vec3,
    config: &ChatterConfig,
    // Add terrain query parameters here when terrain system is available
) -> f32 {
    let mut interference = 1.0;
    
    // Example terrain checks - replace with actual terrain system integration
    let height = position.y;
    if height > 100.0 {
        // In mountainous area
        interference *= config.terrain_interference.mountain_factor;
    }
    
    // Weather impact
    interference *= config.terrain_interference.weather_factor;
    
    interference
}

/// System to update AI trucker transmissions
fn update_ai_transmissions(
    time: Res<Time>,
    mut truckers: Query<(&mut AITrucker, &Transform)>,
    mut radio_manager: ResMut<CBRadioManager>,
    config: Res<ChatterConfig>,
) {
    let mut rng = rand::thread_rng();

    for (mut trucker, transform) in truckers.iter_mut() {
        trucker.next_transmission.tick(time.delta());
        
        if trucker.next_transmission.finished() {
            // Calculate interference at current position
            let interference = calculate_interference(
                transform.translation,
                &config,
            );
            
            // Generate a message based on mood and context
            let message_type = if trucker.active_conversation {
                MessageType::Conversation
            } else {
                match rng.gen_range(0..7) {
                    0 => MessageType::TrafficReport,
                    1 => MessageType::WeatherWarning,
                    2 => MessageType::BearReport,
                    3 => MessageType::Location,
                    4 => MessageType::Emergency,
                    5 => MessageType::Conversation,
                    _ => MessageType::General,
                }
            };

            let message = message_type.generate(trucker, &trucker.conversation_state.topic);
            
            // Add transmission to radio manager with interference factor
            radio_manager.add_transmission(19, message, interference);

            // Update trucker state
            if rng.gen_bool(0.2) {
                // Move to next location in route
                trucker.route_index = (trucker.route_index + 1) % trucker.route.len();
                trucker.location = trucker.route[trucker.route_index].clone();
            }

            // Occasionally change mood
            if rng.gen_bool(0.1) {
                trucker.mood = match rng.gen_range(0..5) {
                    0 => TruckerMood::Friendly,
                    1 => TruckerMood::Cautious,
                    2 => TruckerMood::Helpful,
                    3 => TruckerMood::Tired,
                    _ => TruckerMood::Chatty,
                };
            }

            // Set next transmission time based on conversation state
            let next_interval = if trucker.active_conversation {
                rng.gen_range(15.0..45.0) // More frequent during conversations
            } else {
                rng.gen_range(30.0..120.0)
            };
            
            trucker.next_transmission.set_duration(Duration::from_secs_f32(next_interval));
            trucker.next_transmission.reset();
        }
    }
}

/// System to handle AI responses to transmissions
fn handle_responses(
    mut truckers: Query<(&mut AITrucker, &Transform)>,
    radio_manager: Res<CBRadioManager>,
    config: Res<ChatterConfig>,
) {
    let mut rng = rand::thread_rng();

    // Check for recent player transmissions
    if let Some((message, player_pos)) = radio_manager.last_player_transmission() {
        for (mut trucker, transform) in truckers.iter_mut() {
            // Calculate response chance based on distance and interference
            let distance = transform.translation.distance(player_pos);
            let interference = calculate_interference(transform.translation, &config);
            let response_chance = trucker.response_chance * interference * 
                calculate_signal_strength(distance, &radio_manager.config());

            if rng.gen_bool(response_chance as f64) {
                // Generate contextual response based on message content
                trucker.last_heard_message = Some(message.clone());
                trucker.active_conversation = true;

                let response = MessageType::Response.generate(
                    trucker,
                    &trucker.conversation_state.topic
                );
                
                radio_manager.add_transmission(19, response, interference);
                
                // Set a shorter delay for conversation flow
                trucker.next_transmission.set_duration(Duration::from_secs_f32(
                    rng.gen_range(15.0..45.0)
                ));
                trucker.next_transmission.reset();
            }
        }
    } else {
        // Gradually end conversations when no recent transmissions
        for (mut trucker, _) in truckers.iter_mut() {
            if trucker.active_conversation && rng.gen_bool(0.3) {
                trucker.active_conversation = false;
                trucker.last_heard_message = None;
            }
        }
    }
}

/// Channel-specific conversation topics
#[derive(Clone, Debug)]
pub enum ConversationTopic {
    Traffic,
    Weather,
    LawEnforcement,
    Directions,
    TruckStop,
    Maintenance,
    FuelPrices,
    LoadInfo,
    GeneralChat,
    // New topics
    RestArea,           // Rest area conditions and availability
    Restaurants,        // Food and dining recommendations
    RoadConstruction,   // Construction updates
    Scenery,           // Scenic routes and views
    LocalEvents,        // Local events and attractions
    TruckServices,     // Repair shops, towing, etc.
    Regulations,        // Weight stations, regulations
    Parking,           // Parking availability
    EmergencySupport,  // Breakdown assistance
}

impl ConversationTopic {
    fn for_channel(channel: u8) -> Option<Self> {
        match channel {
            9 => Some(Self::LawEnforcement),   // Emergency channel
            19 => Some(Self::Traffic),         // Main trucker channel
            13 => Some(Self::Weather),         // Weather info
            17 => Some(Self::Directions),      // Directions and navigation
            21 => Some(Self::TruckStop),       // Truck stop info
            24 => Some(Self::Maintenance),     // Maintenance and repairs
            27 => Some(Self::FuelPrices),      // Fuel prices and stations
            30 => Some(Self::LoadInfo),        // Load and cargo info
            15 => Some(Self::RestArea),        // Rest area info
            23 => Some(Self::Restaurants),     // Food recommendations
            25 => Some(Self::RoadConstruction),// Construction updates
            28 => Some(Self::TruckServices),   // Service info
            31 => Some(Self::Parking),         // Parking availability
            _ => Some(Self::GeneralChat),
        }
    }
}

/// Active conversation tracking
#[derive(Default)]
pub struct Conversation {
    pub topic: ConversationTopic,
    pub participants: Vec<Entity>,
    pub recent_messages: Vec<String>,
    pub last_message_time: f32,
    pub topic_start_time: f32,
    pub interest_level: f32,
}

impl Conversation {
    fn update_interest_levels(&mut self, truckers: &Query<(Entity, &AITrucker, &Transform)>) -> f32 {
        let mut total_interest = 0.0;
        let mut participant_count = 0;
        
        for &participant in &self.participants {
            if let Ok((_, trucker, _)) = truckers.get(participant) {
                let base_interest = match &trucker.conversation_state {
                    TruckerConversationState::InConversation { interest, .. } => *interest,
                    _ => 0.0,
                };
                
                // Adjust interest based on personality
                let personality_factor = match self.topic {
                    ConversationTopic::GeneralChat => trucker.personality.chattiness,
                    ConversationTopic::EmergencySupport => trucker.personality.helpfulness,
                    ConversationTopic::Maintenance | ConversationTopic::TruckServices => {
                        (trucker.personality.professionalism + trucker.personality.helpfulness) * 0.5
                    },
                    _ => (trucker.personality.chattiness + trucker.personality.patience) * 0.5,
                };
                
                total_interest += base_interest * personality_factor;
                participant_count += 1;
            }
        }
        
        self.interest_level = if participant_count > 0 {
            total_interest / participant_count as f32
        } else {
            0.0
        };
        
        self.interest_level
    }
    
    fn should_continue(&self, world_state: &WorldState, truckers: &Query<(Entity, &AITrucker, &Transform)>) -> bool {
        let time_active = world_state.time_of_day - self.last_message_time;
        
        // Basic conditions for conversation continuation
        let mut should_continue = self.interest_level > 0.2 && time_active < 900.0; // 15 minutes max
        
        // Special cases for certain topics
        match self.topic {
            ConversationTopic::EmergencySupport => {
                // Emergency conversations continue with lower interest if emergency is ongoing
                should_continue = should_continue || (self.interest_level > 0.1 && 
                    world_state.recent_events.iter().any(|e| matches!(e, EventType::Emergency(_))));
            },
            ConversationTopic::Weather => {
                // Weather discussions continue during severe weather
                should_continue = should_continue || matches!(
                    world_state.current_weather,
                    WeatherType::Storm | WeatherType::Snow | WeatherType::Ice
                );
            },
            _ => {}
        }
        
        should_continue
    }

    fn update_conversation_state(&mut self, world_state: &WorldState, truckers: &Query<(Entity, &AITrucker, &Transform)>) {
        // Update interest levels
        self.update_interest_levels(truckers);

        // Check for topic transitions
        let transitions = get_topic_transitions();
        let valid_transitions: Vec<&TopicTransition> = transitions.iter()
            .filter(|t| t.from == self.topic)
            .filter(|t| self.should_transition(world_state, t))
            .collect();

        // Attempt topic transition if conditions are met
        let mut rng = rand::thread_rng();
        for transition in valid_transitions {
            if rng.gen::<f32>() < transition.probability {
                self.topic = transition.to.clone();
                self.topic_start_time = world_state.time_of_day;
                break;
            }
        }

        // Update participant states
        self.participants.retain(|&participant| {
            if let Ok((_, trucker, _)) = truckers.get(participant) {
                matches!(trucker.conversation_state, TruckerConversationState::InConversation { .. })
            } else {
                false
            }
        });
    }

    fn add_message(&mut self, message: String, world_state: &WorldState) {
        self.recent_messages.push(message);
        self.last_message_time = world_state.time_of_day;
        
        // Keep only last 10 messages
        if self.recent_messages.len() > 10 {
            self.recent_messages.remove(0);
        }
    }
}

/// System to manage channel-specific conversations
fn manage_conversation_flow(
    mut commands: Commands,
    mut truckers: Query<(Entity, &mut AITrucker, &Transform)>,
    time: Res<Time>,
    radio_manager: Res<CBRadioManager>,
    world_state: Res<WorldState>,
) {
    // Track active conversations
    let mut active_conversations = Vec::new();
    
    // Group truckers by their current conversations
    for (entity, trucker, _) in truckers.iter() {
        if let TruckerConversationState::InConversation { topic, participants, .. } = &trucker.conversation_state {
            // Find or create conversation
            let conv_idx = active_conversations.iter().position(|c: &Conversation| {
                c.participants.contains(&entity)
            });
            
            if let Some(idx) = conv_idx {
                // Update existing conversation
                let conv = &mut active_conversations[idx];
                if !conv.participants.contains(&entity) {
                    conv.participants.push(entity);
                }
            } else {
                // Start new conversation
                active_conversations.push(Conversation {
                    topic: topic.clone(),
                    participants: vec![entity],
                    recent_messages: Vec::new(),
                    last_message_time: time.elapsed_seconds(),
                    topic_start_time: time.elapsed_seconds(),
                    interest_level: 0.0,
                });
            }
        }
    }
    
    // Update conversations and check for transitions
    for conv in &mut active_conversations {
        if conv.try_transition(&world_state) {
            // Notify participants of topic change
            for &participant in &conv.participants {
                if let Ok((_, mut trucker, _)) = truckers.get_mut(participant) {
                    if let TruckerConversationState::InConversation { ref mut topic, .. } = trucker.conversation_state {
                        *topic = conv.topic.clone();
                    }
                }
            }
        }
    }
    
    // Clean up inactive conversations
    active_conversations.retain(|conv| {
        time.elapsed_seconds() - conv.last_message_time < 300.0 // 5 minute timeout
            && !conv.participants.is_empty()
    });
}

// Helper function to generate contextual responses
fn generate_contextual_response(
    trucker: &AITrucker,
    last_message: &str,
    topic: &ConversationTopic,
    interest: f32,
) -> String {
    let mut rng = rand::thread_rng();
    
    // Extract keywords and sentiment from last message
    let keywords = extract_keywords(last_message);
    let sentiment = analyze_sentiment(last_message);
    
    // Generate response based on context
    match topic {
        ConversationTopic::Traffic => {
            if keywords.contains(&"alternate".to_string()) {
                // Suggest alternate route
                if let Some(knowledge) = trucker.road_knowledge.get(&trucker.location) {
                    if !knowledge.alternate_routes.is_empty() {
                        format!("{} here. You can try going through {}. Took that route {} ago, wasn't too bad.",
                            trucker.handle,
                            knowledge.alternate_routes[0],
                            format_time(current_time - knowledge.reported_time)
                        )
                    }
                }
            }
            // ... more context-specific responses
        },
        // ... handle other topics
    }
}

// Helper function to extract keywords from a message
fn extract_keywords(message: &str) -> Vec<String> {
    message.to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

// Helper function to analyze message sentiment
fn analyze_sentiment(message: &str) -> f32 {
    // Simple sentiment analysis
    let positive_words = ["good", "clear", "thanks", "helpful", "appreciate"];
    let negative_words = ["bad", "avoid", "careful", "warning", "dangerous"];
    
    let words = message.to_lowercase().split_whitespace();
    let mut sentiment = 0.5;
    
    for word in words {
        if positive_words.contains(&word) {
            sentiment += 0.1;
        } else if negative_words.contains(&word) {
            sentiment -= 0.1;
        }
    }
    
    sentiment.clamp(0.0, 1.0)
}

// Helper function to format time differences
fn format_time(seconds: f32) -> String {
    if seconds < 60.0 {
        "just now".to_string()
    } else if seconds < 3600.0 {
        format!("{} minutes", (seconds / 60.0).round())
    } else {
        format!("{} hours", (seconds / 3600.0).round())
    }
}

// Enhanced road knowledge system
#[derive(Clone, Debug)]
pub struct RoadKnowledge {
    condition_type: RoadConditionType,
    severity: f32,        // 0.0-1.0
    reported_time: f32,
    location: String,
    reporter: String,     // Handle of the trucker who reported it
    confirmed_by: Vec<String>, // Handles of truckers who confirmed this info
    details: String,      // Additional details
    estimated_duration: Option<f32>, // Estimated duration in hours
    alternate_routes: Vec<String>,   // Suggested alternate routes
}

#[derive(Clone, Debug, PartialEq)]
pub enum RoadConditionType {
    Traffic(TrafficType),
    Construction(ConstructionType),
    Weather(WeatherType),
    Hazard(HazardType),
    LawEnforcement(EnforcementType),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrafficType {
    Congestion,
    Accident,
    SlowMoving,
    Stopped,
    MergeDelay,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConstructionType {
    LaneClosure,
    RoadWork,
    BridgeRepair,
    Detour,
    Paving,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WeatherType {
    Rain,
    Snow,
    Ice,
    Fog,
    Wind,
    Storm,
    Clear,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HazardType {
    Debris,
    SpilledLoad,
    BrokenVehicle,
    Wildlife,
    RoadDamage,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EnforcementType {
    SpeedTrap,
    WeighStation,
    Checkpoint,
    Patrol,
    Inspection,
}

// Enhanced conversation state management
#[derive(Clone, Debug)]
pub enum ConversationState {
    Listening,
    Responding {
        to_handle: String,
        context: String,
        topic: ConversationTopic,
    },
    Initiating {
        topic: ConversationTopic,
        urgency: f32,
    },
    InConversation {
        topic: ConversationTopic,
        participants: Vec<String>,
        last_spoke: f32,
        interest: f32,
        context_chain: Vec<String>,
        turn_index: usize,
    },
    Confirming {
        knowledge: RoadKnowledge,
        original_reporter: String,
    },
}

// System to share and update road knowledge
fn share_road_knowledge(
    mut commands: Commands,
    mut truckers: Query<(Entity, &mut AITrucker, &Transform)>,
    time: Res<Time>,
    mut radio_manager: ResMut<CBRadioManager>,
) {
    let current_time = time.elapsed_seconds();
    let mut rng = rand::thread_rng();

    // Collect and verify knowledge
    let mut verified_knowledge = Vec::new();
    for (_, trucker, _) in truckers.iter() {
        for (location, knowledge) in trucker.road_knowledge.iter() {
            if knowledge.confirmed_by.len() >= 2 && 
               current_time - knowledge.reported_time < 3600.0 { // Within last hour
                verified_knowledge.push((location.clone(), knowledge.clone()));
            }
        }
    }

    // Share important knowledge
    for (entity, mut trucker, transform) in truckers.iter_mut() {
        for (location, knowledge) in verified_knowledge.iter() {
            // Check if trucker doesn't have this knowledge
            if !trucker.road_knowledge.contains_key(location) {
                // Calculate relevance based on route and location
                let is_relevant = trucker.route.contains(location) || 
                                location == &trucker.location;
                
                if is_relevant && rng.gen_bool(0.7) { // 70% chance to share relevant info
                    let message = format_knowledge_message(&knowledge, &trucker.personality);
                    let interference = calculate_interference(transform.translation, &radio_manager.config());
                    radio_manager.add_transmission(19, message, interference);
                    
                    // Update trucker's knowledge
                    trucker.road_knowledge.insert(location.clone(), knowledge.clone());
                }
            }
        }
    }
}

// Helper function to format knowledge sharing messages
fn format_knowledge_message(knowledge: &RoadKnowledge, personality: &TruckerPersonality) -> String {
    let mut rng = rand::thread_rng();
    
    let intro = if personality.professionalism > 0.7 {
        "Confirmed report"
    } else if personality.humor > 0.7 {
        "Heads up good buddies"
    } else {
        "Just heard"
    };

    let condition = match &knowledge.condition_type {
        RoadConditionType::Traffic(t) => format!("traffic {}", match t {
            TrafficType::Congestion => "backing up",
            TrafficType::Accident => "accident clearing",
            TrafficType::SlowMoving => "moving slow",
            TrafficType::Stopped => "at a standstill",
            TrafficType::MergeDelay => "merge delays",
        }),
        RoadConditionType::Construction(c) => format!("construction {}", match c {
            ConstructionType::LaneClosure => "closing lanes",
            ConstructionType::RoadWork => "road work ongoing",
            ConstructionType::BridgeRepair => "bridge repairs",
            ConstructionType::Detour => "detour in place",
            ConstructionType::Paving => "paving operation",
        }),
        // ... similar matches for other condition types
    };

    let severity = if knowledge.severity > 0.8 {
        "major"
    } else if knowledge.severity > 0.5 {
        "moderate"
    } else {
        "minor"
    };

    let alt_route = if !knowledge.alternate_routes.is_empty() {
        format!(" Alternate route via {}.", 
            knowledge.alternate_routes[rng.gen_range(0..knowledge.alternate_routes.len())])
    } else {
        String::new()
    };

    format!("{}: {} {} at {}. Severity: {}.{} Confirmed by {} drivers.",
        intro,
        condition,
        knowledge.details,
        knowledge.location,
        severity,
        alt_route,
        knowledge.confirmed_by.len()
    )
}

#[derive(Clone, Debug)]
pub struct TruckerPersonality {
    chattiness: f32,      // 0.0-1.0: How likely to initiate conversations
    helpfulness: f32,     // 0.0-1.0: How likely to respond to questions
    patience: f32,        // 0.0-1.0: How long to stay in conversations
    humor: f32,          // 0.0-1.0: How likely to use humor
    professionalism: f32, // 0.0-1.0: How formal in communication
}

impl Default for TruckerPersonality {
    fn default() -> Self {
        Self {
            chattiness: 0.5,
            helpfulness: 0.5,
            patience: 0.5,
            humor: 0.5,
            professionalism: 0.5,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TruckerConversationState {
    Listening,
    Responding(String),   // Response context
    Initiating(ConversationTopic),
    InConversation {
        topic: ConversationTopic,
        last_spoke: f32,
        interest: f32,    // 0.0-1.0: Current interest in conversation
    },
}

#[derive(Clone, Debug)]
pub struct RoadCondition {
    condition_type: RoadConditionType,
    severity: f32,        // 0.0-1.0
    reported_time: f32,
    location: String,
}

// Add weather and emergency-related components
#[derive(Clone, Debug)]
pub struct WeatherReport {
    weather_type: WeatherType,
    severity: f32,
    visibility: f32,
    wind_speed: f32,
    precipitation: f32,
    location: String,
    reported_time: f32,
    affected_areas: Vec<String>,
    moving_direction: Option<Vec3>,
    expected_duration: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct EmergencyReport {
    emergency_type: EmergencyType,
    severity: f32,
    location: String,
    reported_time: f32,
    details: String,
    responders_notified: bool,
    needs_assistance: bool,
    reporter: String,
    confirmed_by: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EmergencyType {
    MedicalEmergency,
    VehicleBreakdown,
    HazardousSpill,
    RoadClosure,
    SevereWeather,
    AccidentScene,
    SearchAndRescue,
}

// Add event chain handling
#[derive(Clone, Debug)]
pub struct EventChain {
    events: Vec<ChainedEvent>,
    start_time: f32,
    duration: f32,
    active: bool,
    participants: Vec<String>,
    resolution_status: ResolutionStatus,
}

#[derive(Clone, Debug)]
pub struct ChainedEvent {
    event_type: EventType,
    location: String,
    severity: f32,
    timestamp: f32,
    responses: Vec<EventResponse>,
    next_events: Vec<EventProbability>,
}

#[derive(Clone, Debug)]
pub struct EventResponse {
    responder: String,
    message: String,
    timestamp: f32,
    response_type: ResponseType,
}

#[derive(Clone, Debug)]
pub struct EventProbability {
    event: ChainedEvent,
    probability: f32,
    conditions: Vec<EventCondition>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventType {
    Traffic(TrafficEvent),
    Weather(WeatherEvent),
    Emergency(EmergencyEvent),
    Social(SocialEvent),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrafficEvent {
    InitialBackup,
    AccidentReported,
    EmergencyResponse,
    TrafficClearing,
    RoadReopening,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WeatherEvent {
    WeatherApproaching,
    ConditionsWorsening,
    VisibilityDropping,
    WeatherPeaking,
    ConditionsImproving,
    // New weather events
    ThunderstormForming,
    TornadoWarning,
    FlashFloodWarning,
    BlackIceWarning,
    DustStormApproaching,
    HighWindWarning,
    FreezingRainWarning,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EmergencyEvent {
    InitialReport,
    FirstResponder,
    BackupRequested,
    EmergencyServicesArrival,
    SceneSecured,
    TrafficControl,
    CleanupInProgress,
    AllClear,
    // New specialized events
    MedicalAssistance {
        severity: f32,
        conscious: bool,
        breathing: bool,
    },
    VehicleRecovery {
        vehicle_type: String,
        blocking_traffic: bool,
        tow_requested: bool,
    },
    HazmatIncident {
        material_type: String,
        containment_status: bool,
        evacuation_needed: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum SocialEvent {
    ConvoyForming,
    RestStopMeetup,
    LocalAdvice,
    GroupDiscussion,
    SharedMeal,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResponseType {
    Acknowledgment,
    Assistance,
    Information,
    Warning,
    Confirmation,
    Question,
    Resolution,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolutionStatus {
    Ongoing,
    NeedsAssistance,
    UnderControl,
    Resolved,
    Abandoned,
}

#[derive(Debug, Clone)]
pub enum EventCondition {
    TimeElapsed(f32),
    MinParticipants(usize),
    WeatherCondition(WeatherType),
    LocationReached(String),
    ResponseReceived(String),
}

// Add to AITrucker implementation
impl AITrucker {
    pub fn handle_weather_report(&mut self, report: &WeatherReport) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        // Update trucker's knowledge of weather conditions
        if self.route.iter().any(|loc| report.affected_areas.contains(loc)) {
            // Generate response based on personality and relevance
            let response = match self.personality.professionalism {
                p if p > 0.7 => {
                    format!(
                        "{} confirming {} conditions at {}. Visibility at {}%, wind speed {} mph. {}",
                        self.handle,
                        report.weather_type.to_string(),
                        report.location,
                        (report.visibility * 100.0) as i32,
                        report.wind_speed as i32,
                        if self.personality.helpfulness > 0.6 {
                            format!("Recommend reducing speed and increasing following distance.")
                        } else {
                            String::new()
                        }
                    )
                },
                _ => {
                    let conditions = match report.weather_type {
                        WeatherType::Rain if report.severity > 0.7 => "raining cats and dogs",
                        WeatherType::Snow if report.severity > 0.7 => "white-out conditions",
                        WeatherType::Fog if report.visibility < 0.3 => "thick as pea soup",
                        WeatherType::Wind if report.wind_speed > 30.0 => "blowing like crazy",
                        _ => "getting rough",
                    };
                    format!("{} here, can confirm it's {} at {}.", self.handle, conditions, report.location)
                }
            };
            Some(response)
        } else {
            None
        }
    }

    pub fn handle_emergency(&mut self, emergency: &EmergencyReport) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        // Check if trucker is near the emergency
        let is_nearby = self.location == emergency.location || 
                       self.route.contains(&emergency.location);
        
        if is_nearby {
            // Generate appropriate response based on emergency type and personality
            let response = match emergency.emergency_type {
                EmergencyType::MedicalEmergency => {
                    if self.personality.helpfulness > 0.7 {
                        format!(
                            "This is {} responding to medical emergency at {}. I'm {} away and can assist if needed.",
                            self.handle,
                            emergency.location,
                            self.estimate_distance_to(&emergency.location)
                        )
                    } else {
                        format!(
                            "{} confirming medical emergency at {}. Emergency services notified.",
                            self.handle,
                            emergency.location
                        )
                    }
                },
                EmergencyType::VehicleBreakdown => {
                    if self.personality.helpfulness > 0.6 {
                        format!(
                            "{} here. I've got tools and can stop to help with breakdown at {}.",
                            self.handle,
                            emergency.location
                        )
                    } else {
                        format!(
                            "Copy that breakdown at {}. Watch for stopped vehicle.",
                            emergency.location
                        )
                    }
                },
                _ => format!(
                    "{} acknowledging {} at {}. {}",
                    self.handle,
                    emergency.emergency_type.to_string(),
                    emergency.location,
                    if emergency.needs_assistance {
                        "Standing by to assist if needed."
                    } else {
                        "Use caution in the area."
                    }
                ),
            };
            Some(response)
        } else {
            None
        }
    }

    fn estimate_distance_to(&self, location: &str) -> String {
        // TODO: Implement actual distance calculation
        "about 10 miles".to_string()
    }

    pub fn handle_event_chain(&mut self, event_chain: &EventChain) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        // Check if trucker should participate
        if !event_chain.participants.contains(&self.handle) &&
           !self.should_join_event(event_chain) {
            return None;
        }

        // Get the latest event in the chain
        if let Some(current_event) = event_chain.events.last() {
            // Generate response based on event type and trucker personality
            let response = match &current_event.event_type {
                EventType::Traffic(traffic_event) => {
                    self.generate_traffic_chain_response(traffic_event, event_chain)
                },
                EventType::Weather(weather_event) => {
                    self.generate_weather_chain_response(weather_event, event_chain)
                },
                EventType::Emergency(emergency_event) => {
                    self.generate_emergency_chain_response(emergency_event, event_chain)
                },
                EventType::Social(social_event) => {
                    self.generate_social_chain_response(social_event, event_chain)
                },
            };

            // Add trucker to participants if not already involved
            if !event_chain.participants.contains(&self.handle) {
                event_chain.participants.push(self.handle.clone());
            }

            response
        } else {
            None
        }
    }

    fn should_join_event(&self, event_chain: &EventChain) -> bool {
        let mut rng = rand::thread_rng();
        
        // Calculate base chance based on personality
        let base_chance = match event_chain.events.last() {
            Some(event) => match event.event_type {
                EventType::Emergency(_) => self.personality.helpfulness,
                EventType::Social(_) => self.personality.chattiness,
                EventType::Traffic(_) => self.personality.professionalism,
                EventType::Weather(_) => self.personality.helpfulness * self.personality.professionalism,
            },
            None => 0.0,
        };

        // Modify chance based on current activity and location
        let location_factor = if self.is_near_event(event_chain) { 2.0 } else { 0.5 };
        let activity_factor = if self.active_conversation { 0.5 } else { 1.0 };
        
        rng.gen_bool((base_chance * location_factor * activity_factor) as f64)
    }

    fn is_near_event(&self, event_chain: &EventChain) -> bool {
        if let Some(event) = event_chain.events.last() {
            // TODO: Implement actual distance checking
            self.location == event.location || self.route.contains(&event.location)
        } else {
            false
        }
    }

    fn generate_traffic_chain_response(&self, event: &TrafficEvent, chain: &EventChain) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        match event {
            TrafficEvent::InitialBackup => {
                Some(format!(
                    "{} here. Traffic's {} at {}. {}",
                    self.handle,
                    if self.personality.professionalism > 0.7 {
                        "experiencing significant congestion"
                    } else {
                        "backing up bad"
                    },
                    chain.events.last()?.location,
                    if self.personality.helpfulness > 0.6 {
                        "Recommend seeking alternate routes."
                    } else {
                        "Watch yourself."
                    }
                ))
            },
            TrafficEvent::AccidentReported => {
                if self.personality.professionalism > 0.7 {
                    Some(format!(
                        "This is {} confirming accident scene at {}. Emergency vehicles en route. Please maintain safe distance.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                } else {
                    Some(format!(
                        "Yeah, {} here. Got a wreck at {}. Bears incoming, keep your distance.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                }
            },
            // ... handle other traffic events
        }
    }

    fn generate_weather_chain_response(&self, event: &WeatherEvent, chain: &EventChain) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        match event {
            WeatherEvent::WeatherApproaching => {
                Some(format!(
                    "{} with a heads up. Got {} approaching from {}. {}",
                    self.handle,
                    if self.personality.professionalism > 0.7 {
                        "adverse weather conditions"
                    } else {
                        "nasty weather"
                    },
                    chain.events.last()?.location,
                    if self.personality.helpfulness > 0.6 {
                        "Recommend preparing for reduced visibility and slick conditions."
                    } else {
                        "Might want to hunker down."
                    }
                ))
            },
            WeatherEvent::ConditionsWorsening => {
                if self.personality.professionalism > 0.7 {
                    Some(format!(
                        "This is {} reporting deteriorating conditions at {}. Visibility continuing to decrease.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                } else {
                    Some(format!(
                        "{} here. It's getting worse by the minute at {}. Can barely see past my hood.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                },
            },
            // ... handle other weather events
        }
    }

    fn generate_emergency_chain_response(&self, event: &EmergencyEvent, chain: &EventChain) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        match event {
            EmergencyEvent::InitialReport => {
                Some(format!(
                    "Break {} - Emergency situation at {}. {}",
                    if self.personality.professionalism > 0.7 {
                        "break, emergency traffic"
                    } else {
                        "one-nine"
                    },
                    chain.events.last()?.location,
                    if self.personality.helpfulness > 0.8 {
                        "I'm approaching scene to assist."
                    } else {
                        "Use caution in the area."
                    }
                ))
            },
            EmergencyEvent::FirstResponder => {
                if self.personality.helpfulness > 0.7 {
                    Some(format!(
                        "{} on scene at {}. Assessing situation and coordinating with emergency services.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                } else {
                    Some(format!(
                        "{} confirming emergency at {}. First responders taking over.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                },
            },
            // ... handle other emergency events
        }
    }

    fn generate_social_chain_response(&self, event: &SocialEvent, chain: &EventChain) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        match event {
            SocialEvent::ConvoyForming => {
                if self.personality.chattiness > 0.7 {
                    Some(format!(
                        "{} here. Forming up a convoy at {}. Anyone headed {} want to join?",
                        self.handle,
                        chain.events.last()?.location,
                        // TODO: Get actual destination from route
                        "eastbound"
                    ))
                } else {
                    None
                }
            },
            SocialEvent::RestStopMeetup => {
                if self.personality.chattiness > 0.6 {
                    Some(format!(
                        "{} stopping at {} for a break. Good food and parking available if anyone's interested.",
                        self.handle,
                        chain.events.last()?.location
                    ))
                } else {
                    None
                },
            },
            // ... handle other social events
        }
    }
}

// Add new system for managing event chains
fn manage_event_chains(
    mut commands: Commands,
    mut truckers: Query<(&mut AITrucker, &Transform)>,
    time: Res<Time>,
    mut world_state: ResMut<WorldState>,
    mut weather_system: ResMut<WeatherSystem>,
    mut radio_manager: ResMut<CBRadioManager>,
) {
    // Update weather system and process front interactions
    weather_system.update(&time, &mut world_state);
    weather_system.process_front_interactions();
    
    // Check for weather alerts
    let current_time = time.elapsed_seconds();
    let alerts = weather_system.check_for_alerts();
    
    // Process weather alerts
    for alert in alerts {
        // Broadcast alert on weather channel (13)
        for (mut trucker, transform) in truckers.iter_mut() {
            if let Some(response) = trucker.handle_weather_alert(&alert) {
                let interference = calculate_interference(
                    transform.translation,
                    &radio_manager.config(),
                ) * weather_system.visibility_factor;
                
                radio_manager.add_transmission(13, response, interference);
            }
        }
        
        // Create weather event chain if severe enough
        if alert.severity == AlertSeverity::Warning {
            let chain = EventChainFactory::create_weather_chain(
                alert.location.clone(),
                1.0,
                match alert.event_type {
                    WeatherEvent::TornadoWarning => WeatherType::Storm,
                    WeatherEvent::FlashFloodWarning => WeatherType::Rain,
                    WeatherEvent::BlackIceWarning => WeatherType::Snow,
                    _ => WeatherType::Storm,
                },
            );
            world_state.active_event_chains.push(chain);
        }
    }
    
    // Continue with existing event chain management...
    // [Previous event chain management code remains unchanged]
}

// Add weather system integration
#[derive(Clone, Debug)]
pub struct WeatherSystem {
    current_conditions: HashMap<String, WeatherCondition>,
    weather_fronts: Vec<WeatherFront>,
    global_conditions: WeatherType,
    visibility_factor: f32,
    update_timer: Timer,
}

#[derive(Clone, Debug)]
pub struct WeatherCondition {
    weather_type: WeatherType,
    severity: f32,
    visibility: f32,
    wind_speed: f32,
    precipitation: f32,
    temperature: f32,
    pressure: f32,
    moving_direction: Option<Vec3>,
    affected_radius: f32,
}

#[derive(Clone, Debug)]
pub struct WeatherFront {
    position: Vec3,
    direction: Vec3,
    speed: f32,
    size: f32,
    intensity: f32,
    weather_type: WeatherType,
    lifetime: Timer,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self {
            current_conditions: HashMap::new(),
            weather_fronts: Vec::new(),
            global_conditions: WeatherType::Clear,
            visibility_factor: 1.0,
            update_timer: Timer::from_seconds(60.0, TimerMode::Repeating),
        }
    }
}

impl WeatherSystem {
    fn update(&mut self, time: &Time, world_state: &mut WorldState) {
        self.update_timer.tick(time.delta());
        
        if self.update_timer.finished() {
            // Update weather fronts
            for front in &mut self.weather_fronts {
                front.position += front.direction * front.speed * time.delta_seconds();
                front.lifetime.tick(time.delta());
                
                // Update conditions for locations affected by the front
                for (location, position) in &world_state.location_positions {
                    let distance = position.distance(front.position);
                    if distance <= front.size {
                        let intensity_factor = 1.0 - (distance / front.size);
                        self.update_location_weather(
                            location,
                            front.weather_type.clone(),
                            front.intensity * intensity_factor
                        );
                    }
                }
                
                // Remove expired fronts
                self.weather_fronts.retain(|f| !f.lifetime.finished());
            }
            
            // Generate new weather fronts
            if self.weather_fronts.len() < 3 && rand::random::<f32>() < 0.3 {
                self.generate_weather_front();
            }
            
            // Update global visibility
            self.update_visibility();
            
            // Update world state
            world_state.current_weather = self.determine_dominant_weather();
        }
    }
    
    fn update_location_weather(&mut self, location: &str, weather_type: WeatherType, intensity: f32) {
        let condition = self.current_conditions.entry(location.to_string())
            .or_insert_with(|| WeatherCondition {
                weather_type: WeatherType::Clear,
                severity: 0.0,
                visibility: 1.0,
                wind_speed: 0.0,
                precipitation: 0.0,
                temperature: 20.0,
                pressure: 1013.0,
                moving_direction: None,
                affected_radius: 1000.0,
            });
        
        // Blend new weather with existing conditions
        condition.severity = (condition.severity + intensity).min(1.0);
        condition.weather_type = weather_type;
        
        // Update related parameters
        match weather_type {
            WeatherType::Rain => {
                condition.visibility *= 0.7;
                condition.precipitation = intensity;
                condition.pressure -= intensity * 5.0;
            },
            WeatherType::Snow => {
                condition.visibility *= 0.5;
                condition.precipitation = intensity;
                condition.temperature = 0.0;
            },
            WeatherType::Fog => {
                condition.visibility *= 0.3;
                condition.wind_speed *= 0.5;
            },
            WeatherType::Storm => {
                condition.visibility *= 0.4;
                condition.wind_speed = 50.0 + intensity * 30.0;
                condition.precipitation = intensity;
                condition.pressure -= intensity * 10.0;
            },
            _ => {},
        }
    }
    
    fn generate_weather_front(&mut self) {
        let mut rng = rand::thread_rng();
        
        // Generate random position on the edge of the map
        let map_size = 5000.0; // Adjust based on your world size
        let (position, direction) = if rng.gen_bool(0.5) {
            // Horizontal movement
            let x = if rng.gen_bool(0.5) { -map_size } else { map_size };
            let z = rng.gen_range(-map_size..map_size);
            (
                Vec3::new(x, 0.0, z),
                Vec3::new(if x < 0.0 { 1.0 } else { -1.0 }, 0.0, 0.0)
            )
        } else {
            // Vertical movement
            let x = rng.gen_range(-map_size..map_size);
            let z = if rng.gen_bool(0.5) { -map_size } else { map_size };
            (
                Vec3::new(x, 0.0, z),
                Vec3::new(0.0, 0.0, if z < 0.0 { 1.0 } else { -1.0 })
            )
        };
        
        // Create weather front
        let weather_type = match rng.gen_range(0..5) {
            0 => WeatherType::Rain,
            1 => WeatherType::Snow,
            2 => WeatherType::Fog,
            3 => WeatherType::Storm,
            _ => WeatherType::Clear,
        };
        
        let front = WeatherFront {
            position,
            direction,
            speed: rng.gen_range(10.0..30.0),
            size: rng.gen_range(1000.0..3000.0),
            intensity: rng.gen_range(0.3..1.0),
            weather_type,
            lifetime: Timer::from_seconds(rng.gen_range(1800.0..7200.0), TimerMode::Once),
        };
        
        self.weather_fronts.push(front);
    }
    
    fn update_visibility(&mut self) {
        let mut total_visibility = 1.0;
        let mut condition_count = 0;
        
        for condition in self.current_conditions.values() {
            total_visibility *= condition.visibility;
            condition_count += 1;
        }
        
        if condition_count > 0 {
            self.visibility_factor = total_visibility.powf(1.0 / condition_count as f32);
        }
    }
    
    fn determine_dominant_weather(&self) -> WeatherType {
        let mut weather_counts: HashMap<WeatherType, f32> = HashMap::new();
        
        for condition in self.current_conditions.values() {
            *weather_counts.entry(condition.weather_type.clone()).or_default() += condition.severity;
        }
        
        weather_counts.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(weather_type, _)| weather_type)
            .unwrap_or(WeatherType::Clear)
    }

    // Add new method for weather alerts
    fn check_for_alerts(&self) -> Vec<WeatherAlert> {
        let mut alerts = Vec::new();
        let mut rng = rand::thread_rng();
        
        for (location, condition) in &self.current_conditions {
            // Check for severe weather conditions
            if condition.severity > 0.8 {
                match condition.weather_type {
                    WeatherType::Storm if condition.wind_speed > 60.0 => {
                        // Potential tornado conditions
                        if rng.gen_bool(0.2) {
                            alerts.push(WeatherAlert {
                                event_type: WeatherEvent::TornadoWarning,
                                severity: AlertSeverity::Warning,
                                location: location.clone(),
                                affected_radius: 2000.0,
                                start_time: 0.0, // Will be set when processed
                                duration: 1800.0, // 30 minutes
                                details: format!("Tornado conditions detected. Wind speeds exceeding {} mph.", 
                                    condition.wind_speed as i32),
                                confirmed_by: Vec::new(),
                            });
                        }
                    },
                    WeatherType::Rain if condition.precipitation > 0.9 => {
                        // Flash flood potential
                        alerts.push(WeatherAlert {
                            event_type: WeatherEvent::FlashFloodWarning,
                            severity: AlertSeverity::Warning,
                            location: location.clone(),
                            affected_radius: 3000.0,
                            start_time: 0.0,
                            duration: 3600.0, // 1 hour
                            details: "Heavy rainfall causing flash flood conditions.".to_string(),
                            confirmed_by: Vec::new(),
                        });
                    },
                    WeatherType::Snow if condition.temperature < 2.0 && condition.precipitation > 0.6 => {
                        // Black ice risk
                        alerts.push(WeatherAlert {
                            event_type: WeatherEvent::BlackIceWarning,
                            severity: AlertSeverity::Warning,
                            location: location.clone(),
                            affected_radius: 5000.0,
                            start_time: 0.0,
                            duration: 7200.0, // 2 hours
                            details: "Near-freezing temperatures and precipitation creating black ice conditions.".to_string(),
                            confirmed_by: Vec::new(),
                        });
                    },
                    _ => {}
                }
            }
            
            // Check for developing conditions
            if condition.severity > 0.6 && condition.severity < 0.8 {
                match condition.weather_type {
                    WeatherType::Storm => {
                        alerts.push(WeatherAlert {
                            event_type: WeatherEvent::ThunderstormForming,
                            severity: AlertSeverity::Watch,
                            location: location.clone(),
                            affected_radius: 4000.0,
                            start_time: 0.0,
                            duration: 3600.0,
                            details: "Conditions favorable for severe thunderstorm development.".to_string(),
                            confirmed_by: Vec::new(),
                        });
                    },
                    WeatherType::Rain if condition.temperature < 4.0 => {
                        alerts.push(WeatherAlert {
                            event_type: WeatherEvent::FreezingRainWarning,
                            severity: AlertSeverity::Watch,
                            location: location.clone(),
                            affected_radius: 4000.0,
                            start_time: 0.0,
                            duration: 3600.0,
                            details: "Potential for freezing rain developing.".to_string(),
                            confirmed_by: Vec::new(),
                        });
                    },
                    _ => {}
                }
            }
        }
        
        alerts
    }

    // Add method for weather front interactions
    fn process_front_interactions(&mut self) {
        let mut interactions = Vec::new();
        let fronts = self.weather_fronts.clone();
        
        // Check for colliding fronts
        for (i, front1) in fronts.iter().enumerate() {
            for (j, front2) in fronts.iter().enumerate() {
                if i != j {
                    let distance = front1.position.distance(front2.position);
                    if distance < (front1.size + front2.size) * 0.5 {
                        interactions.push((i, j));
                    }
                }
            }
        }
        
        // Process interactions
        for (i, j) in interactions {
            let front1 = &self.weather_fronts[i];
            let front2 = &self.weather_fronts[j];
            
            // Create new weather front based on interaction
            let new_front = match (front1.weather_type.clone(), front2.weather_type.clone()) {
                (WeatherType::Rain, WeatherType::Storm) |
                (WeatherType::Storm, WeatherType::Rain) => {
                    // Intensify into severe thunderstorm
                    Some(WeatherFront {
                        position: (front1.position + front2.position) * 0.5,
                        direction: (front1.direction + front2.direction).normalize(),
                        speed: (front1.speed + front2.speed) * 0.7,
                        size: (front1.size + front2.size) * 0.8,
                        intensity: (front1.intensity + front2.intensity).min(1.0),
                        weather_type: WeatherType::Storm,
                        lifetime: Timer::from_seconds(3600.0, TimerMode::Once),
                    })
                },
                (WeatherType::Rain, WeatherType::Snow) |
                (WeatherType::Snow, WeatherType::Rain) => {
                    // Create freezing rain conditions
                    Some(WeatherFront {
                        position: (front1.position + front2.position) * 0.5,
                        direction: (front1.direction + front2.direction).normalize(),
                        speed: (front1.speed + front2.speed) * 0.6,
                        size: (front1.size + front2.size) * 0.9,
                        intensity: (front1.intensity + front2.intensity).min(1.0),
                        weather_type: WeatherType::Rain,
                        lifetime: Timer::from_seconds(2700.0, TimerMode::Once),
                    })
                },
                _ => None,
            };
            
            // Add new front if interaction produced one
            if let Some(front) = new_front {
                self.weather_fronts.push(front);
            }
        }
    }

    fn generate_complex_weather_pattern(&mut self) -> Vec<WeatherFront> {
        let mut rng = rand::thread_rng();
        let mut fronts = Vec::new();
        
        // Generate 1-3 interacting weather fronts
        let num_fronts = rng.gen_range(1..=3);
        
        for _ in 0..num_fronts {
            let position = Vec3::new(
                rng.gen_range(-1000.0..1000.0),
                0.0,
                rng.gen_range(-1000.0..1000.0)
            );
            
            let direction = Vec3::new(
                rng.gen_range(-1.0..1.0),
                0.0,
                rng.gen_range(-1.0..1.0)
            ).normalize();
            
            let weather_type = match rng.gen_range(0..100) {
                0..=15 => WeatherType::Clear,
                16..=35 => WeatherType::Rain,
                36..=50 => WeatherType::Snow,
                51..=65 => WeatherType::Fog,
                66..=80 => WeatherType::Wind,
                81..=95 => WeatherType::Storm,
                _ => WeatherType::Ice,
            };
            
            let front = WeatherFront {
                position,
                direction,
                speed: rng.gen_range(10.0..30.0),
                size: rng.gen_range(100.0..500.0),
                intensity: rng.gen_range(0.2..0.9),
                weather_type,
                lifetime: Timer::from_seconds(rng.gen_range(300.0..1800.0), TimerMode::Once),
            };
            
            fronts.push(front);
        }
        
        fronts
    }

    fn process_front_interactions(&mut self) {
        let mut interactions = Vec::new();
        let fronts = self.weather_fronts.clone();
        
        // Check each pair of fronts for interactions
        for (i, front1) in fronts.iter().enumerate() {
            for (j, front2) in fronts.iter().enumerate() {
                if i >= j { continue; }
                
                let distance = (front1.position - front2.position).length();
                let interaction_threshold = (front1.size + front2.size) * 0.5;
                
                if distance < interaction_threshold {
                    let interaction = self.calculate_front_interaction(front1, front2);
                    interactions.push((i, j, interaction));
                }
            }
        }
        
        // Apply interactions
        for (i, j, interaction) in interactions {
            if let Some(front1) = self.weather_fronts.get_mut(i) {
                front1.intensity = (front1.intensity + interaction.intensity_change).clamp(0.1, 1.0);
                front1.size = (front1.size + interaction.size_change).max(50.0);
                if let Some(new_type) = interaction.new_weather_type {
                    front1.weather_type = new_type;
                }
            }
            
            if let Some(front2) = self.weather_fronts.get_mut(j) {
                front2.intensity = (front2.intensity + interaction.intensity_change).clamp(0.1, 1.0);
                front2.size = (front2.size + interaction.size_change).max(50.0);
                if let Some(new_type) = interaction.new_weather_type {
                    front2.weather_type = new_type;
                }
            }
        }
    }

    fn calculate_front_interaction(&self, front1: &WeatherFront, front2: &WeatherFront) -> WeatherInteraction {
        let mut rng = rand::thread_rng();
        
        // Base changes
        let intensity_change = rng.gen_range(-0.2..0.2);
        let size_change = rng.gen_range(-50.0..100.0);
        
        // Determine if weather types should combine
        let new_weather_type = match (front1.weather_type, front2.weather_type) {
            (WeatherType::Rain, WeatherType::Wind) | 
            (WeatherType::Wind, WeatherType::Rain) if front1.intensity > 0.7 && front2.intensity > 0.7 => {
                Some(WeatherType::Storm)
            },
            (WeatherType::Rain, WeatherType::Snow) |
            (WeatherType::Snow, WeatherType::Rain) if front1.intensity > 0.5 && front2.intensity > 0.5 => {
                Some(WeatherType::Ice)
            },
            _ => None,
        };
        
        WeatherInteraction {
            intensity_change,
            size_change,
            new_weather_type,
        }
    }
}

#[derive(Debug)]
struct WeatherInteraction {
    intensity_change: f32,
    size_change: f32,
    new_weather_type: Option<WeatherType>,
}

// Update the world state system
#[derive(Resource)]
pub struct WorldState {
    time_of_day: f32,
    current_weather: WeatherType,
    active_participants: usize,
    participant_locations: HashMap<String, Vec3>,
    location_positions: HashMap<String, Vec3>,
    emergency_services_responding: bool,
    recent_events: Vec<EventType>,
    active_event_chains: Vec<EventChain>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            time_of_day: 12.0, // Noon
            current_weather: WeatherType::Rain,
            active_participants: 0,
            participant_locations: HashMap::new(),
            location_positions: HashMap::new(),
            emergency_services_responding: false,
            recent_events: Vec::new(),
            active_event_chains: Vec::new(),
        }
    }
}

// Add event chain factory
pub struct EventChainFactory;

impl EventChainFactory {
    pub fn create_traffic_chain(location: String, severity: f32) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Traffic(TrafficEvent::InitialBackup),
            location: location.clone(),
            severity,
            timestamp: 0.0, // Will be set when added
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Traffic(TrafficEvent::AccidentReported),
                        location: location.clone(),
                        severity: severity * 1.2,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.4,
                    conditions: vec![
                        EventCondition::MinParticipants(3),
                    ],
                },
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Traffic(TrafficEvent::TrafficClearing),
                        location: location.clone(),
                        severity: severity * 0.5,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.3,
                    conditions: vec![
                        EventCondition::TimeOfDay(0.0, 24.0), // Any time
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0, // Will be set when added
            duration: 3600.0, // 1 hour default
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }

    pub fn create_weather_chain(location: String, severity: f32, weather_type: WeatherType) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Weather(WeatherEvent::WeatherApproaching),
            location: location.clone(),
            severity,
            timestamp: 0.0,
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Weather(WeatherEvent::ConditionsWorsening),
                        location: location.clone(),
                        severity: severity * 1.3,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: vec![
                            EventProbability {
                                event: ChainedEvent {
                                    event_type: EventType::Weather(WeatherEvent::WeatherPeaking),
                                    location: location.clone(),
                                    severity: severity * 1.5,
                                    timestamp: 0.0,
                                    responses: Vec::new(),
                                    next_events: Vec::new(),
                                },
                                probability: 0.7,
                                conditions: vec![
                                    EventCondition::MinParticipants(2),
                                ],
                            },
                        ],
                    },
                    probability: 0.6,
                    conditions: vec![
                        EventCondition::WeatherCondition(weather_type.clone()),
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0,
            duration: 7200.0, // 2 hours for weather events
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }

    pub fn create_emergency_chain(location: String, severity: f32, emergency_type: EmergencyType) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Emergency(EmergencyEvent::InitialReport),
            location: location.clone(),
            severity,
            timestamp: 0.0,
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::FirstResponder),
                        location: location.clone(),
                        severity,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: vec![
                            EventProbability {
                                event: ChainedEvent {
                                    event_type: EventType::Emergency(EmergencyEvent::BackupRequested),
                                    location: location.clone(),
                                    severity: severity * 1.2,
                                    timestamp: 0.0,
                                    responses: Vec::new(),
                                    next_events: Vec::new(),
                                },
                                probability: 0.5,
                                conditions: vec![
                                    EventCondition::MinParticipants(1),
                                ],
                            },
                        ],
                    },
                    probability: 0.8,
                    conditions: vec![
                        EventCondition::LocationProximity(location.clone(), 1000.0), // Within 1000 units
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0,
            duration: 1800.0, // 30 minutes for emergency events
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }

    pub fn create_social_chain(location: String, social_type: SocialEvent) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Social(social_type.clone()),
            location: location.clone(),
            severity: 0.5, // Social events have moderate severity
            timestamp: 0.0,
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Social(SocialEvent::GroupDiscussion),
                        location: location.clone(),
                        severity: 0.6,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.7,
                    conditions: vec![
                        EventCondition::MinParticipants(3),
                        EventCondition::TimeOfDay(6.0, 22.0), // Daytime hours
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0,
            duration: 3600.0, // 1 hour for social events
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }

    pub fn create_medical_emergency_chain(location: String, severity: f32) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Emergency(EmergencyEvent::MedicalAssistance {
                severity,
                conscious: true,
                breathing: true,
            }),
            location: location.clone(),
            severity,
            timestamp: 0.0,
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::EmergencyServicesArrival),
                        location: location.clone(),
                        severity,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.8,
                    conditions: vec![
                        EventCondition::TimeElapsed(300.0), // 5 minutes
                        EventCondition::MinParticipants(1),
                    ],
                },
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::SceneSecured),
                        location: location.clone(),
                        severity: severity * 0.8,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.6,
                    conditions: vec![
                        EventCondition::TimeElapsed(120.0), // 2 minutes
                        EventCondition::MinParticipants(2),
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0,
            duration: 1800.0, // 30 minutes
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }

    pub fn create_hazmat_emergency_chain(location: String, severity: f32) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Emergency(EmergencyEvent::HazmatIncident {
                material_type: "Unknown".to_string(),
                containment_status: false,
                evacuation_needed: severity > 0.7,
            }),
            location: location.clone(),
            severity,
            timestamp: 0.0,
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::EmergencyServicesArrival),
                        location: location.clone(),
                        severity,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.9,
                    conditions: vec![
                        EventCondition::TimeElapsed(600.0), // 10 minutes
                        EventCondition::MinParticipants(1),
                    ],
                },
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::AreaSecured),
                        location: location.clone(),
                        severity: severity * 0.9,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.7,
                    conditions: vec![
                        EventCondition::TimeElapsed(300.0), // 5 minutes
                        EventCondition::MinParticipants(3),
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0,
            duration: 3600.0, // 1 hour
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }

    pub fn create_vehicle_recovery_chain(location: String, severity: f32) -> EventChain {
        let initial_event = ChainedEvent {
            event_type: EventType::Emergency(EmergencyEvent::VehicleRecovery {
                vehicle_type: "Truck".to_string(),
                blocking_traffic: severity > 0.5,
                tow_requested: true,
            }),
            location: location.clone(),
            severity,
            timestamp: 0.0,
            responses: Vec::new(),
            next_events: vec![
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::TrafficControl),
                        location: location.clone(),
                        severity: severity * 0.8,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.7,
                    conditions: vec![
                        EventCondition::TimeElapsed(180.0), // 3 minutes
                        EventCondition::MinParticipants(2),
                    ],
                },
                EventProbability {
                    event: ChainedEvent {
                        event_type: EventType::Emergency(EmergencyEvent::TowTruckArrived),
                        location: location.clone(),
                        severity: severity * 0.6,
                        timestamp: 0.0,
                        responses: Vec::new(),
                        next_events: Vec::new(),
                    },
                    probability: 0.8,
                    conditions: vec![
                        EventCondition::TimeElapsed(1200.0), // 20 minutes
                        EventCondition::MinParticipants(1),
                    ],
                },
            ],
        };

        EventChain {
            events: vec![initial_event],
            start_time: 0.0,
            duration: 2400.0, // 40 minutes
            active: true,
            participants: Vec::new(),
            resolution_status: ResolutionStatus::Ongoing,
        }
    }
}

// Update the world state system
fn update_world_state(
    mut world_state: ResMut<WorldState>,
    time: Res<Time>,
    truckers: Query<(&AITrucker, &Transform)>,
    config: Res<ChatterConfig>,
) {
    // Update time of day (24-hour cycle)
    world_state.time_of_day = (time.elapsed_seconds() % 86400.0) / 3600.0;

    // Update participant tracking
    world_state.participant_locations.clear();
    world_state.active_participants = 0;

    for (trucker, transform) in truckers.iter() {
        if trucker.active_conversation {
            world_state.active_participants += 1;
            world_state.participant_locations.insert(trucker.handle.clone(), transform.translation);
        }
    }

    // Update location positions if needed
    if world_state.location_positions.is_empty() {
        for location in &config.locations {
            // TODO: Implement proper location positioning
            world_state.location_positions.insert(
                location.clone(),
                Vec3::new(
                    rand::random::<f32>() * 1000.0,
                    0.0,
                    rand::random::<f32>() * 1000.0,
                ),
            );
        }
    }

    // Maintain recent events list (keep last 10)
    while world_state.recent_events.len() > 10 {
        world_state.recent_events.remove(0);
    }
}

// Update AITrucker to handle weather alerts
impl AITrucker {
    pub fn handle_weather_alert(&mut self, alert: &WeatherAlert) -> Option<String> {
        let mut rng = rand::thread_rng();
        
        // Check if trucker is in affected area
        let is_affected = self.location == alert.location || 
                         self.route.iter().any(|loc| loc == &alert.location);
        
        if is_affected {
            let urgency_prefix = match alert.severity {
                AlertSeverity::Warning => "URGENT WEATHER WARNING!",
                AlertSeverity::Watch => "Weather Watch Alert:",
                AlertSeverity::Advisory => "Weather Advisory:",
            };
            
            let response = match self.personality.professionalism {
                p if p > 0.7 => {
                    format!(
                        "{} {} {} Reported at {} with {} mile radius affected. {}",
                        urgency_prefix,
                        alert.event_type.to_string(),
                        alert.details,
                        alert.location,
                        (alert.affected_radius / 1609.0) as i32, // Convert to miles
                        if self.personality.helpfulness > 0.6 {
                            "Please exercise extreme caution and reduce speed."
                        } else {
                            ""
                        }
                    )
                },
                _ => {
                    let condition_desc = match alert.event_type {
                        WeatherEvent::TornadoWarning => "twister forming",
                        WeatherEvent::FlashFloodWarning => "water rising fast",
                        WeatherEvent::BlackIceWarning => "roads are like glass",
                        WeatherEvent::ThunderstormForming => "nasty storm brewing",
                        WeatherEvent::FreezingRainWarning => "freezing rain coming down",
                        _ => "weather's getting dangerous",
                    };
                    format!(
                        "Break break! Got {} at {}! {}",
                        condition_desc,
                        alert.location,
                        if self.personality.helpfulness > 0.5 {
                            "Better find somewhere safe to wait it out."
                        } else {
                            "Watch yourselves out there."
                        }
                    )
                }
            };
            Some(response)
        } else {
            None
        }
    }
}

/// Represents a conversation topic transition
#[derive(Clone, Debug)]
pub struct TopicTransition {
    from: ConversationTopic,
    to: ConversationTopic,
    probability: f32,
    conditions: Vec<TransitionCondition>,
}

/// Conditions that can trigger a topic transition
#[derive(Clone, Debug)]
pub enum TransitionCondition {
    TimeElapsed(f32),              // Minimum time in current topic
    ParticipantCount(usize),       // Minimum participants needed
    KeywordMentioned(String),      // Keyword that triggers transition
    EventOccurred(EventType),      // Event that triggers transition
    InterestLevel(f32),            // Minimum interest level needed
    WeatherChange(WeatherType),    // Weather condition that triggers transition
}

impl Conversation {
    fn should_transition(&self, world_state: &WorldState, transition: &TopicTransition) -> bool {
        let mut valid = true;
        
        for condition in &transition.conditions {
            valid &= match condition {
                TransitionCondition::TimeElapsed(min_time) => {
                    world_state.time_of_day - self.last_message_time >= *min_time
                },
                TransitionCondition::ParticipantCount(min_count) => {
                    self.participants.len() >= *min_count
                },
                TransitionCondition::KeywordMentioned(keyword) => {
                    self.messages.iter().any(|msg| msg.to_lowercase().contains(&keyword.to_lowercase()))
                },
                TransitionCondition::EventOccurred(event_type) => {
                    world_state.recent_events.contains(event_type)
                },
                TransitionCondition::InterestLevel(min_interest) => {
                    // Calculate average interest from participants
                    let avg_interest = self.calculate_average_interest();
                    avg_interest >= *min_interest
                },
                TransitionCondition::WeatherChange(weather) => {
                    world_state.current_weather == *weather
                },
            };
            
            if !valid {
                break;
            }
        }
        
        valid
    }
    
    fn calculate_average_interest(&self) -> f32 {
        // Implementation would need access to AITrucker components
        // This is a placeholder
        0.5
    }
    
    fn try_transition(&mut self, world_state: &WorldState) -> bool {
        let available_transitions = get_topic_transitions();
        
        // Filter transitions that start from current topic
        let valid_transitions: Vec<&TopicTransition> = available_transitions.iter()
            .filter(|t| t.from == self.topic)
            .collect();
            
        // Check conditions and probabilities
        let mut rng = rand::thread_rng();
        for transition in valid_transitions {
            if self.should_transition(world_state, transition) && rng.gen::<f32>() < transition.probability {
                self.topic = transition.to.clone();
                return true;
            }
        }
        
        false
    }

    fn update_interest_levels(&mut self, truckers: &Query<(Entity, &AITrucker, &Transform)>) -> f32 {
        let mut total_interest = 0.0;
        let mut participant_count = 0;
        
        for &participant in &self.participants {
            if let Ok((_, trucker, _)) = truckers.get(participant) {
                let base_interest = match &trucker.conversation_state {
                    TruckerConversationState::InConversation { interest, .. } => *interest,
                    _ => 0.0,
                };
                
                // Adjust interest based on personality
                let personality_factor = match self.topic {
                    ConversationTopic::GeneralChat => trucker.personality.chattiness,
                    ConversationTopic::EmergencySupport => trucker.personality.helpfulness,
                    ConversationTopic::Maintenance | ConversationTopic::TruckServices => {
                        (trucker.personality.professionalism + trucker.personality.helpfulness) * 0.5
                    },
                    _ => (trucker.personality.chattiness + trucker.personality.patience) * 0.5,
                };
                
                total_interest += base_interest * personality_factor;
                participant_count += 1;
            }
        }
        
        if participant_count > 0 {
            total_interest / participant_count as f32
        } else {
            0.0
        }
    }
    
    fn should_continue(&self, world_state: &WorldState, truckers: &Query<(Entity, &AITrucker, &Transform)>) -> bool {
        let avg_interest = self.update_interest_levels(truckers);
        let time_active = world_state.time_of_day - self.last_message_time;
        
        // Basic conditions for conversation continuation
        let mut should_continue = avg_interest > 0.2 && time_active < 900.0; // 15 minutes max
        
        // Special cases for certain topics
        match self.topic {
            ConversationTopic::EmergencySupport => {
                // Emergency conversations continue with lower interest if emergency is ongoing
                should_continue = should_continue || (avg_interest > 0.1 && 
                    world_state.recent_events.iter().any(|e| matches!(e, EventType::Emergency(_))));
            },
            ConversationTopic::Weather => {
                // Weather discussions continue during severe weather
                should_continue = should_continue || matches!(
                    world_state.current_weather,
                    WeatherType::Storm | WeatherType::Snow | WeatherType::Ice
                );
            },
            _ => {}
        }
        
        should_continue
    }

    fn update_conversation_state(&mut self, world_state: &WorldState, truckers: &Query<(Entity, &AITrucker, &Transform)>) {
        // Update last active time if there are messages
        if !self.messages.is_empty() {
            self.last_message_time = world_state.time_of_day;
        }

        // Check for topic transitions
        let transitions = get_topic_transitions();
        let valid_transitions: Vec<&TopicTransition> = transitions.iter()
            .filter(|t| t.from == self.topic)
            .filter(|t| t.should_transition(world_state, &self))
            .collect();

        // Attempt topic transition if conditions are met
        for transition in valid_transitions {
            if transition.try_transition() {
                self.topic = transition.to;
                self.topic_start_time = world_state.time_of_day;
                break;
            }
        }

        // Update participant states
        self.participants.retain(|&participant| {
            if let Ok((_, trucker, _)) = truckers.get(participant) {
                matches!(trucker.conversation_state, TruckerConversationState::InConversation { .. })
            } else {
                false
            }
        });
    }

    fn add_participant(&mut self, participant: Entity, world_state: &WorldState) {
        if !self.participants.contains(&participant) {
            self.participants.push(participant);
            self.last_message_time = world_state.time_of_day;
        }
    }

    fn remove_participant(&mut self, participant: Entity) {
        if let Some(pos) = self.participants.iter().position(|&p| p == participant) {
            self.participants.remove(pos);
        }
    }

    fn add_message(&mut self, message: CBMessage, world_state: &WorldState) {
        self.messages.push(message);
        self.last_message_time = world_state.time_of_day;
        
        // Keep only last 10 messages
        if self.messages.len() > 10 {
            self.messages.remove(0);
        }
    }
}

fn get_topic_transitions() -> Vec<TopicTransition> {
    vec![
        // Weather-related transitions
        TopicTransition {
            from: ConversationTopic::GeneralChat,
            to: ConversationTopic::Weather,
            probability: 0.7,
            conditions: vec![
                TransitionCondition::WeatherChange(WeatherType::Storm),
                TransitionCondition::TimeElapsed(300.0), // 5 minutes
            ],
        },
        // Traffic-related transitions
        TopicTransition {
            from: ConversationTopic::GeneralChat,
            to: ConversationTopic::Traffic,
            probability: 0.8,
            conditions: vec![
                TransitionCondition::KeywordMentioned("backup".to_string()),
                TransitionCondition::ParticipantCount(2),
            ],
        },
        // Emergency-related transitions
        TopicTransition {
            from: ConversationTopic::Traffic,
            to: ConversationTopic::EmergencySupport,
            probability: 0.9,
            conditions: vec![
                TransitionCondition::EventOccurred(EventType::Emergency(EmergencyEvent::InitialReport)),
            ],
        },
        // More natural transitions
        TopicTransition {
            from: ConversationTopic::Weather,
            to: ConversationTopic::RoadConstruction,
            probability: 0.6,
            conditions: vec![
                TransitionCondition::TimeElapsed(180.0), // 3 minutes
                TransitionCondition::InterestLevel(0.3),
            ],
        },
        // Rest area discussions
        TopicTransition {
            from: ConversationTopic::GeneralChat,
            to: ConversationTopic::RestArea,
            probability: 0.5,
            conditions: vec![
                TransitionCondition::TimeElapsed(600.0), // 10 minutes
                TransitionCondition::KeywordMentioned("tired".to_string()),
            ],
        },
        // Food and restaurant recommendations
        TopicTransition {
            from: ConversationTopic::RestArea,
            to: ConversationTopic::Restaurants,
            probability: 0.6,
            conditions: vec![
                TransitionCondition::TimeElapsed(120.0), // 2 minutes
                TransitionCondition::ParticipantCount(2),
            ],
        },
        // Local events discussion
        TopicTransition {
            from: ConversationTopic::Restaurants,
            to: ConversationTopic::LocalEvents,
            probability: 0.4,
            conditions: vec![
                TransitionCondition::InterestLevel(0.6),
                TransitionCondition::ParticipantCount(3),
            ],
        },
        // Maintenance discussions
        TopicTransition {
            from: ConversationTopic::GeneralChat,
            to: ConversationTopic::Maintenance,
            probability: 0.5,
            conditions: vec![
                TransitionCondition::KeywordMentioned("engine".to_string()),
                TransitionCondition::KeywordMentioned("repair".to_string()),
            ],
        },
        // Truck services
        TopicTransition {
            from: ConversationTopic::Maintenance,
            to: ConversationTopic::TruckServices,
            probability: 0.7,
            conditions: vec![
                TransitionCondition::TimeElapsed(240.0), // 4 minutes
                TransitionCondition::InterestLevel(0.4),
            ],
        },
        // Regulations and compliance
        TopicTransition {
            from: ConversationTopic::TruckServices,
            to: ConversationTopic::Regulations,
            probability: 0.5,
            conditions: vec![
                TransitionCondition::KeywordMentioned("inspection".to_string()),
                TransitionCondition::ParticipantCount(2),
            ],
        },
    ]
}

impl EventChainManager {
    pub fn update_emergency_chains(&mut self, world: &World) {
        let time = world.resource::<Time>();
        let current_time = time.elapsed_seconds();
        
        for chain in &mut self.active_chains {
            if !chain.active {
                continue;
            }
            
            // Check if chain has expired
            if current_time - chain.start_time > chain.duration {
                chain.active = false;
                chain.resolution_status = ResolutionStatus::TimedOut;
                continue;
            }
            
            // Update events in the chain
            for event in &mut chain.events {
                // Check conditions for next events
                let mut new_events = Vec::new();
                
                for prob in &event.next_events {
                    if self.check_event_conditions(world, chain, &prob.conditions) 
                        && rand::thread_rng().gen::<f32>() < prob.probability {
                        let mut next_event = prob.event.clone();
                        next_event.timestamp = current_time;
                        new_events.push(next_event);
                    }
                }
                
                // Add triggered events to the chain
                for new_event in new_events {
                    chain.events.push(new_event);
                    
                    // Generate responses from nearby truckers for the new event
                    let truckers = world.query::<&AITrucker>();
                    for trucker in truckers.iter() {
                        if chain.participants.contains(&trucker.handle) 
                            && rand::thread_rng().gen::<f32>() < trucker.personality.chattiness {
                            if let Some(response) = trucker.generate_emergency_response(&new_event.event_type, chain) {
                                event.responses.push(EventResponse {
                                    responder: trucker.handle.clone(),
                                    message: response,
                                    timestamp: current_time,
                                    response_type: ResponseType::Update,
                                });
                            }
                        }
                    }
                }
            }
            
            // Check if chain has been resolved
            if chain.events.last().map_or(false, |e| matches!(e.event_type, 
                EventType::Emergency(EmergencyEvent::AllClear | EmergencyEvent::SceneSecured))) {
                chain.active = false;
                chain.resolution_status = ResolutionStatus::Resolved;
            }
        }
        
        // Clean up inactive chains
        self.active_chains.retain(|chain| chain.active);
    }
    
    fn check_event_conditions(&self, world: &World, chain: &EventChain, conditions: &[EventCondition]) -> bool {
        let time = world.resource::<Time>();
        let current_time = time.elapsed_seconds();
        
        for condition in conditions {
            match condition {
                EventCondition::TimeElapsed(duration) => {
                    if current_time - chain.start_time < *duration {
                        return false;
                    }
                },
                EventCondition::MinParticipants(count) => {
                    if chain.participants.len() < *count {
                        return false;
                    }
                },
                EventCondition::WeatherCondition(required_weather) => {
                    if let Some(weather_system) = world.get_resource::<WeatherSystem>() {
                        if weather_system.current_weather.weather_type != *required_weather {
                            return false;
                        }
                    }
                },
                EventCondition::LocationReached(location) => {
                    let truckers = world.query::<&AITrucker>();
                    let mut reached = false;
                    for trucker in truckers.iter() {
                        if chain.participants.contains(&trucker.handle) 
                            && trucker.estimate_distance_to(location) < 100.0 {
                            reached = true;
                            break;
                        }
                    }
                    if !reached {
                        return false;
                    }
                },
                EventCondition::ResponseReceived(handle) => {
                    let mut found = false;
                    for event in &chain.events {
                        for response in &event.responses {
                            if &response.responder == handle {
                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                    }
                    if !found {
                        return false;
                    }
                },
            }
        }
        true
    }
}

#[derive(Resource)]
pub struct EmergencyServicesAI {
    /// Active emergency units
    units: Vec<EmergencyUnit>,
    /// Pending dispatch requests
    dispatch_queue: VecDeque<DispatchRequest>,
    /// Response time statistics
    response_stats: ResponseStats,
    /// Current resource allocation
    resource_allocation: HashMap<String, Vec<EmergencyUnit>>,
}

#[derive(Clone, Debug)]
pub struct EmergencyUnit {
    unit_id: String,
    unit_type: EmergencyUnitType,
    status: UnitStatus,
    position: Vec3,
    destination: Option<Vec3>,
    estimated_arrival: Option<f32>,
    current_incident: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EmergencyUnitType {
    Police,
    FireTruck,
    Ambulance,
    TowTruck,
    HazmatTeam,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnitStatus {
    Available,
    Dispatched,
    OnScene,
    Returning,
    OutOfService,
}

#[derive(Clone, Debug)]
pub struct DispatchRequest {
    incident_id: String,
    location: Vec3,
    emergency_type: EmergencyType,
    severity: f32,
    timestamp: f32,
    required_units: Vec<EmergencyUnitType>,
}

#[derive(Default)]
struct ResponseStats {
    total_responses: usize,
    average_response_time: f32,
    successful_resolutions: usize,
}

impl Default for EmergencyServicesAI {
    fn default() -> Self {
        Self {
            units: vec![
                // Initialize with some default emergency units
                EmergencyUnit {
                    unit_id: "POL-1".to_string(),
                    unit_type: EmergencyUnitType::Police,
                    status: UnitStatus::Available,
                    position: Vec3::new(0.0, 0.0, 0.0),
                    destination: None,
                    estimated_arrival: None,
                    current_incident: None,
                },
                EmergencyUnit {
                    unit_id: "AMB-1".to_string(),
                    unit_type: EmergencyUnitType::Ambulance,
                    status: UnitStatus::Available,
                    position: Vec3::new(100.0, 0.0, 100.0),
                    destination: None,
                    estimated_arrival: None,
                    current_incident: None,
                },
                EmergencyUnit {
                    unit_id: "TOW-1".to_string(),
                    unit_type: EmergencyUnitType::TowTruck,
                    status: UnitStatus::Available,
                    position: Vec3::new(-100.0, 0.0, -100.0),
                    destination: None,
                    estimated_arrival: None,
                    current_incident: None,
                },
            ],
            dispatch_queue: VecDeque::new(),
            response_stats: ResponseStats::default(),
            resource_allocation: HashMap::new(),
        }
    }
}

impl EmergencyServicesAI {
    pub fn handle_emergency(&mut self, emergency: &EmergencyEvent, location: Vec3, time: f32) -> Option<String> {
        let (emergency_type, severity) = match emergency {
            EmergencyEvent::MedicalAssistance { severity, conscious, breathing } => {
                let severity = if !*breathing {
                    1.0
                } else if !*conscious {
                    0.8
                } else {
                    *severity
                };
                (EmergencyType::MedicalEmergency, severity)
            },
            EmergencyEvent::VehicleRecovery { blocking_traffic, .. } => {
                (EmergencyType::VehicleBreakdown, if *blocking_traffic { 0.8 } else { 0.5 })
            },
            EmergencyEvent::HazmatIncident { evacuation_needed, .. } => {
                (EmergencyType::HazmatSpill, if *evacuation_needed { 1.0 } else { 0.7 })
            },
            _ => return None,
        };

        let incident_id = format!("INC-{}-{}", time as i32, rand::random::<u16>());
        
        // Determine required units based on emergency type and severity
        let required_units = self.determine_required_units(&emergency_type, severity);
        
        // Create dispatch request
        let request = DispatchRequest {
            incident_id: incident_id.clone(),
            location,
            emergency_type,
            severity,
            timestamp: time,
            required_units,
        };
        
        // Add to dispatch queue
        self.dispatch_queue.push_back(request);
        
        // Return dispatch confirmation message
        Some(format!(
            "Emergency Services Dispatch: {} units responding to {} at {}. ETA: {} minutes. Incident ID: {}",
            self.get_unit_type_names(&required_units),
            emergency_type.to_string(),
            self.format_location(location),
            self.estimate_response_time(location),
            incident_id
        ))
    }

    fn determine_required_units(&self, emergency_type: &EmergencyType, severity: f32) -> Vec<EmergencyUnitType> {
        let mut units = Vec::new();
        
        match emergency_type {
            EmergencyType::MedicalEmergency => {
                units.push(EmergencyUnitType::Ambulance);
                if severity > 0.7 {
                    units.push(EmergencyUnitType::Police);
                }
            },
            EmergencyType::VehicleBreakdown => {
                units.push(EmergencyUnitType::TowTruck);
                if severity > 0.6 {
                    units.push(EmergencyUnitType::Police);
                }
            },
            EmergencyType::HazmatSpill => {
                units.push(EmergencyUnitType::HazmatTeam);
                units.push(EmergencyUnitType::FireTruck);
                if severity > 0.8 {
                    units.push(EmergencyUnitType::Police);
                    units.push(EmergencyUnitType::Ambulance);
                }
            },
            _ => {}
        }
        
        units
    }

    fn get_unit_type_names(&self, units: &[EmergencyUnitType]) -> String {
        let names: Vec<_> = units.iter().map(|unit| match unit {
            EmergencyUnitType::Police => "Police",
            EmergencyUnitType::FireTruck => "Fire",
            EmergencyUnitType::Ambulance => "EMS",
            EmergencyUnitType::TowTruck => "Tow",
            EmergencyUnitType::HazmatTeam => "HAZMAT",
        }).collect();
        
        names.join(", ")
    }

    fn format_location(&self, location: Vec3) -> String {
        format!("Mile {:.1} ({:.1}, {:.1})", 
            location.z / 1609.34, // Convert to miles
            location.x,
            location.z
        )
    }

    fn estimate_response_time(&self, location: Vec3) -> i32 {
        // Find closest available unit
        let closest_distance = self.units.iter()
            .filter(|unit| unit.status == UnitStatus::Available)
            .map(|unit| unit.position.distance(location))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(5000.0);
        
        // Estimate time based on distance (assuming average speed of 60 mph)
        let minutes = (closest_distance / 1609.34 / 60.0 * 60.0) as i32;
        minutes.max(1)
    }

    pub fn update(&mut self, time: f32, world_state: &mut WorldState) {
        // Process dispatch queue
        while let Some(request) = self.dispatch_queue.front() {
            if let Some(allocated_units) = self.allocate_units(request) {
                // Update unit statuses and positions
                for unit in allocated_units {
                    if let Some(unit) = self.units.iter_mut().find(|u| u.unit_id == unit.unit_id) {
                        unit.status = UnitStatus::Dispatched;
                        unit.destination = Some(request.location);
                        unit.estimated_arrival = Some(time + self.estimate_response_time(request.location) as f32 * 60.0);
                        unit.current_incident = Some(request.incident_id.clone());
                    }
                }
                
                // Add to resource allocation tracking
                self.resource_allocation.insert(
                    request.incident_id.clone(),
                    allocated_units
                );
                
                // Remove processed request
                self.dispatch_queue.pop_front();
            } else {
                // Couldn't allocate units, leave in queue
                break;
            }
        }
        
        // Update unit positions and status
        for unit in &mut self.units {
            if let (Some(dest), Some(eta)) = (unit.destination, unit.estimated_arrival) {
                if time >= eta {
                    // Unit has arrived
                    unit.position = dest;
                    unit.status = UnitStatus::OnScene;
                    unit.estimated_arrival = None;
                } else {
                    // Update position along path
                    let progress = (time - (eta - self.estimate_response_time(dest) as f32 * 60.0)) 
                        / (self.estimate_response_time(dest) as f32 * 60.0);
                    unit.position = unit.position.lerp(dest, progress.clamp(0.0, 1.0));
                }
            }
        }
        
        // Update response statistics
        self.update_statistics(time);
    }

    fn allocate_units(&self, request: &DispatchRequest) -> Option<Vec<EmergencyUnit>> {
        let mut allocated = Vec::new();
        
        // Try to allocate each required unit type
        for required_type in &request.required_units {
            if let Some(unit) = self.find_available_unit(required_type) {
                allocated.push(unit);
            } else {
                // If we can't allocate all required units, return None
                return None;
            }
        }
        
        Some(allocated)
    }

    fn find_available_unit(&self, unit_type: &EmergencyUnitType) -> Option<EmergencyUnit> {
        self.units.iter()
            .filter(|unit| &unit.unit_type == unit_type && unit.status == UnitStatus::Available)
            .min_by(|a, b| {
                let dist_a = a.position.length();
                let dist_b = b.position.length();
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    fn update_statistics(&mut self, time: f32) {
        // Update response time statistics
        let mut total_time = 0.0;
        let mut responses = 0;
        
        for (_, units) in &self.resource_allocation {
            for unit in units {
                if let Some(incident) = &unit.current_incident {
                    if let Some(request) = self.dispatch_queue.iter().find(|r| r.incident_id == *incident) {
                        total_time += time - request.timestamp;
                        responses += 1;
                    }
                }
            }
        }
        
        if responses > 0 {
            self.response_stats.average_response_time = total_time / responses as f32;
        }
        
        self.response_stats.total_responses = responses;
    }
}

// Add the emergency services update system
fn update_emergency_services(
    mut emergency_services: ResMut<EmergencyServicesAI>,
    mut world_state: ResMut<WorldState>,
    time: Res<Time>,
) {
    emergency_services.update(time.elapsed_seconds(), &mut world_state);
} 