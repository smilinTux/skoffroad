use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use crate::game::audio::cb_radio::{CBRadio, CBRadioManager, Channel};
use crate::game::player::Player;
use crate::terrain::TerrainType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyType {
    Accident,
    MedicalEmergency,
    VehicleFire,
    RoadHazard,
    WeatherEmergency,
    SearchAndRescue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyUnitType {
    Ambulance,
    FireTruck,
    PoliceCar,
    RescueVehicle,
    TowTruck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitStatus {
    Available,
    Dispatched,
    OnScene,
    Returning,
    OutOfService,
    Responding,
}

#[derive(Component)]
pub struct EmergencyUnit {
    pub unit_type: EmergencyUnitType,
    pub status: UnitStatus,
    pub current_location: Vec3,
    pub destination: Option<Vec3>,
    pub response_time: Duration,
    pub experience_level: f32,
}

#[derive(Debug)]
pub struct DispatchRequest {
    pub emergency_type: EmergencyType,
    pub location: Vec3,
    pub severity: f32,
    pub time_received: Duration,
    pub required_units: Vec<EmergencyUnitType>,
    pub assigned_units: Vec<Entity>,
    pub status: RequestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    UnitsAssigned,
    UnitsEnRoute,
    OnScene,
    Resolved,
    Cancelled,
}

#[derive(Resource)]
pub struct EmergencyServicesAI {
    active_units: HashMap<Entity, EmergencyUnitType>,
    dispatch_requests: VecDeque<DispatchRequest>,
    response_statistics: HashMap<EmergencyType, ResponseStats>,
    resource_allocation: HashMap<EmergencyType, Vec<EmergencyUnitType>>,
}

pub struct ResponseStats {
    average_response_time: Duration,
    successful_responses: u32,
    failed_responses: u32,
}

#[derive(Component)]
pub struct EmergencyChannel {
    channel: Channel,
    priority_level: u8,
    active_emergency: Option<EmergencyType>,
}

#[derive(Component)]
pub struct FirstResponderRole {
    role_type: EmergencyUnitType,
    experience: f32,
    completed_missions: u32,
}

#[derive(Resource)]
pub struct EmergencyScenarioGenerator {
    active_scenarios: Vec<EmergencyScenario>,
    scenario_templates: Vec<ScenarioTemplate>,
    last_generation_time: f64,
}

#[derive(Clone, Debug)]
pub struct EmergencyScenario {
    scenario_type: EmergencyType,
    stages: Vec<ScenarioStage>,
    current_stage: usize,
    start_time: f64,
    completion_time: Option<f64>,
    affected_area: Vec3,
    radius: f32,
}

#[derive(Clone, Debug)]
pub struct ScenarioStage {
    description: String,
    required_actions: Vec<RequiredAction>,
    completion_conditions: Vec<CompletionCondition>,
    time_limit: Option<f32>,
}

#[derive(Clone, Debug)]
pub enum RequiredAction {
    ArriveAtLocation(Vec3),
    SecureArea(f32), // radius
    ProvideAssistance(AssistanceType),
    CoordinateWithUnit(EmergencyUnitType),
    ClearHazard(HazardType),
}

#[derive(Clone, Debug)]
pub enum CompletionCondition {
    TimeElapsed(f32),
    UnitsPresent(Vec<EmergencyUnitType>),
    HazardCleared(HazardType),
    VictimsAssisted(u32),
    AreaSecured(Vec3, f32),
}

#[derive(Clone, Debug)]
pub enum AssistanceType {
    MedicalAid,
    Evacuation,
    Traffic,
    FireSuppression,
    TechnicalRescue,
}

#[derive(Clone, Debug)]
pub enum HazardType {
    Fire,
    ChemicalSpill,
    StructuralDamage,
    RoadBlockage,
    WeatherHazard,
}

#[derive(Clone, Debug)]
pub struct ScenarioTemplate {
    emergency_type: EmergencyType,
    base_stages: Vec<ScenarioStage>,
    difficulty_modifiers: Vec<DifficultyModifier>,
}

#[derive(Clone, Debug)]
pub enum DifficultyModifier {
    TimeConstraint(f32),
    AdditionalHazards(Vec<HazardType>),
    WeatherCondition(WeatherType),
    MultipleVictims(u32),
    ComplexTerrain(TerrainType),
}

#[derive(Clone, Debug)]
pub enum WeatherType {
    Clear,
    Rain,
    Storm,
    Snow,
    Fog,
}

#[derive(Component, Debug)]
pub struct EmergencyUnitTracker {
    pub unit_type: EmergencyUnitType,
    pub current_status: UnitStatus,
    pub current_location: Vec3,
    pub destination: Option<Vec3>,
    pub assigned_scenario: Option<Entity>,
    pub experience_points: u32,
    pub successful_missions: u32,
}

impl EmergencyUnitTracker {
    pub fn new(unit_type: EmergencyUnitType) -> Self {
        Self {
            unit_type,
            current_status: UnitStatus::Available,
            current_location: Vec3::ZERO,
            destination: None,
            assigned_scenario: None,
            experience_points: 0,
            successful_missions: 0,
        }
    }

    pub fn update_status(&mut self, new_status: UnitStatus) {
        self.current_status = new_status;
    }

    pub fn assign_to_scenario(&mut self, scenario: Entity, destination: Vec3) {
        self.assigned_scenario = Some(scenario);
        self.destination = Some(destination);
        self.current_status = UnitStatus::Responding;
    }

    pub fn complete_mission(&mut self) {
        self.successful_missions += 1;
        self.experience_points += 10;
        self.assigned_scenario = None;
        self.destination = None;
        self.current_status = UnitStatus::Available;
    }
}

impl EmergencyServicesAI {
    pub fn new() -> Self {
        let mut resource_allocation = HashMap::new();
        
        // Define default unit requirements for each emergency type
        resource_allocation.insert(EmergencyType::Accident, 
            vec![EmergencyUnitType::Ambulance, EmergencyUnitType::PoliceCar]);
        resource_allocation.insert(EmergencyType::MedicalEmergency, 
            vec![EmergencyUnitType::Ambulance]);
        resource_allocation.insert(EmergencyType::VehicleFire, 
            vec![EmergencyUnitType::FireTruck, EmergencyUnitType::PoliceCar]);
        resource_allocation.insert(EmergencyType::RoadHazard, 
            vec![EmergencyUnitType::PoliceCar, EmergencyUnitType::TowTruck]);
        resource_allocation.insert(EmergencyType::WeatherEmergency, 
            vec![EmergencyUnitType::RescueVehicle, EmergencyUnitType::PoliceCar]);
        resource_allocation.insert(EmergencyType::SearchAndRescue, 
            vec![EmergencyUnitType::RescueVehicle, EmergencyUnitType::Ambulance]);

        Self {
            active_units: HashMap::new(),
            dispatch_requests: VecDeque::new(),
            response_statistics: HashMap::new(),
            resource_allocation,
        }
    }

    pub fn handle_emergency(&mut self, emergency_type: EmergencyType, location: Vec3, severity: f32) -> DispatchRequest {
        let required_units = self.determine_required_units(emergency_type, severity);
        let request = DispatchRequest {
            emergency_type,
            location,
            severity,
            time_received: Duration::from_secs(0), // TODO: Get actual game time
            required_units,
            assigned_units: Vec::new(),
            status: RequestStatus::Pending,
        };
        self.dispatch_requests.push_back(request.clone());
        request
    }

    fn determine_required_units(&self, emergency_type: EmergencyType, severity: f32) -> Vec<EmergencyUnitType> {
        let mut required_units = self.resource_allocation
            .get(&emergency_type)
            .cloned()
            .unwrap_or_default();

        // Add additional units based on severity
        if severity > 0.7 {
            match emergency_type {
                EmergencyType::Accident => {
                    required_units.push(EmergencyUnitType::FireTruck);
                }
                EmergencyType::MedicalEmergency => {
                    required_units.push(EmergencyUnitType::PoliceCar);
                }
                _ => {}
            }
        }

        required_units
    }

    fn find_available_unit(&self, unit_type: EmergencyUnitType, query: &Query<(Entity, &EmergencyUnit)>) -> Option<Entity> {
        query.iter()
            .find(|(_, unit)| unit.unit_type == unit_type && unit.status == UnitStatus::Available)
            .map(|(entity, _)| entity)
    }

    fn estimate_response_time(&self, unit_location: Vec3, emergency_location: Vec3) -> Duration {
        // Simple distance-based estimation
        let distance = unit_location.distance(emergency_location);
        let average_speed = 20.0; // meters per second
        let estimated_seconds = distance / average_speed;
        Duration::from_secs_f32(estimated_seconds)
    }

    pub fn handle_priority_message(&mut self, entity: Entity, emergency_type: &EmergencyType) {
        // Update dispatch priorities
        for request in self.dispatch_requests.iter_mut() {
            if request.emergency_type == *emergency_type {
                request.priority += 1;
            }
        }

        // Sort dispatch queue by priority
        self.dispatch_requests.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn notify_unit_arrived(&mut self, unit: Entity) {
        // Update scenario progress when units arrive
        if let Some(request) = self.dispatch_requests.iter_mut()
            .find(|r| r.assigned_units.contains(&unit)) {
            request.units_on_scene += 1;
        }
    }

    pub fn is_scenario_complete(&self, scenario: Entity) -> bool {
        // Check if all required actions for the scenario have been completed
        if let Some(request) = self.dispatch_requests.iter()
            .find(|r| r.scenario_entity == scenario) {
            return request.completion_percentage >= 100.0;
        }
        false
    }

    pub fn get_assignment_for_unit(&self, unit: &EmergencyUnitTracker) -> Option<(Entity, Vec3)> {
        // Find a suitable assignment for an available unit
        self.dispatch_requests.iter()
            .filter(|r| r.required_units.contains(&unit.unit_type))
            .filter(|r| r.assigned_units.len() < r.required_units.len())
            .map(|r| (r.scenario_entity, r.location))
            .next()
    }
}

pub fn update_emergency_units(
    mut units: Query<(Entity, &mut EmergencyUnitTracker, &mut Transform)>,
    time: Res<Time>,
    mut emergency_services: ResMut<EmergencyServicesAI>,
) {
    for (entity, mut unit, mut transform) in units.iter_mut() {
        match unit.current_status {
            UnitStatus::Responding => {
                if let Some(destination) = unit.destination {
                    let direction = destination - transform.translation;
                    if direction.length() < 1.0 {
                        // Unit has arrived at destination
                        unit.update_status(UnitStatus::OnScene);
                        emergency_services.notify_unit_arrived(entity);
                    } else {
                        // Move unit towards destination
                        let movement = direction.normalize() * 30.0 * time.delta_seconds();
                        transform.translation += movement;
                    }
                }
            }
            UnitStatus::OnScene => {
                // Check if the scenario is complete
                if let Some(scenario) = unit.assigned_scenario {
                    if emergency_services.is_scenario_complete(scenario) {
                        unit.complete_mission();
                    }
                }
            }
            UnitStatus::Available => {
                // Check for new assignments
                if let Some((scenario, destination)) = emergency_services.get_assignment_for_unit(&unit) {
                    unit.assign_to_scenario(scenario, destination);
                }
            }
            _ => {}
        }
    }
}

pub fn process_dispatch_requests(
    mut emergency_services: ResMut<EmergencyServicesAI>,
    mut query: Query<(Entity, &mut EmergencyUnit)>,
) {
    let mut requests_to_remove = Vec::new();

    for (i, request) in emergency_services.dispatch_requests.iter().enumerate() {
        if request.status == RequestStatus::Pending {
            let mut all_units_found = true;

            for unit_type in &request.required_units {
                if let Some(unit_entity) = emergency_services.find_available_unit(*unit_type, &query) {
                    if let Ok((_, mut unit)) = query.get_mut(unit_entity) {
                        unit.status = UnitStatus::Dispatched;
                        unit.destination = Some(request.location);
                        unit.response_time = emergency_services.estimate_response_time(
                            unit.current_location,
                            request.location
                        );
                    }
                } else {
                    all_units_found = false;
                    break;
                }
            }

            if all_units_found {
                requests_to_remove.push(i);
            }
        }
    }

    // Remove processed requests
    for &i in requests_to_remove.iter().rev() {
        emergency_services.dispatch_requests.remove(i);
    }
}

pub fn update_emergency_status(
    mut query: Query<(Entity, &EmergencyUnit)>,
    mut emergency_services: ResMut<EmergencyServicesAI>,
) {
    // Update response statistics based on unit statuses
    for (entity, unit) in query.iter() {
        if unit.status == UnitStatus::OnScene {
            if let Some(stats) = emergency_services.response_statistics.get_mut(&EmergencyType::Accident) {
                stats.successful_responses += 1;
                // Update average response time
                let new_avg = (stats.average_response_time.as_secs_f32() * (stats.successful_responses - 1) as f32
                    + unit.response_time.as_secs_f32()) / stats.successful_responses as f32;
                stats.average_response_time = Duration::from_secs_f32(new_avg);
            }
        }
    }
}

pub struct EmergencyServicesPlugin;

impl Plugin for EmergencyServicesPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EmergencyServicesAI>()
            .init_resource::<EmergencyScenarioGenerator>()
            .add_systems(Update, (
                update_first_responder_roles,
                process_emergency_communications,
                update_emergency_units,
                process_dispatch_requests,
                update_emergency_status,
                update_emergency_scenarios,
            ));
    }
}

impl EmergencyScenarioGenerator {
    pub fn new() -> Self {
        Self {
            active_scenarios: Vec::new(),
            scenario_templates: Self::create_default_templates(),
            last_generation_time: 0.0,
        }
    }

    fn create_default_templates() -> Vec<ScenarioTemplate> {
        vec![
            ScenarioTemplate {
                emergency_type: EmergencyType::VehicleAccident,
                base_stages: vec![
                    ScenarioStage {
                        description: "Initial response and scene assessment".to_string(),
                        required_actions: vec![
                            RequiredAction::ArriveAtLocation(Vec3::ZERO),
                            RequiredAction::SecureArea(50.0),
                        ],
                        completion_conditions: vec![
                            CompletionCondition::UnitsPresent(vec![EmergencyUnitType::PoliceCar, EmergencyUnitType::Ambulance]),
                            CompletionCondition::AreaSecured(Vec3::ZERO, 50.0),
                        ],
                        time_limit: Some(300.0),
                    },
                    // Add more stages...
                ],
                difficulty_modifiers: vec![
                    DifficultyModifier::TimeConstraint(0.8),
                    DifficultyModifier::AdditionalHazards(vec![HazardType::Fire]),
                ],
            },
            // Add more templates...
        ]
    }

    pub fn generate_scenario(&mut self, world_state: &WorldState) -> EmergencyScenario {
        let template = self.scenario_templates.choose(&mut rand::thread_rng()).unwrap();
        let location = self.determine_scenario_location(world_state);
        
        EmergencyScenario {
            scenario_type: template.emergency_type.clone(),
            stages: template.base_stages.clone(),
            current_stage: 0,
            start_time: world_state.current_time,
            completion_time: None,
            affected_area: location,
            radius: 100.0,
        }
    }

    fn determine_scenario_location(&self, world_state: &WorldState) -> Vec3 {
        // Implementation for choosing a suitable location based on world state
        // This is a placeholder that should be replaced with actual logic
        Vec3::new(0.0, 0.0, 0.0)
    }
}

// Player integration systems
pub fn update_first_responder_roles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut FirstResponderRole, &Player)>,
    emergency_services: Res<EmergencyServicesAI>,
) {
    for (entity, mut role, _player) in query.iter_mut() {
        // Update experience based on completed missions
        if role.completed_missions > 0 {
            role.experience += 0.1 * role.completed_missions as f32;
            role.completed_missions = 0;
        }

        // Check for role progression
        if role.experience >= 10.0 {
            // Upgrade role capabilities
            commands.entity(entity).insert(EmergencyChannel {
                channel: Channel::Emergency,
                priority_level: 2,
                active_emergency: None,
            });
        }
    }
}

pub fn process_emergency_communications(
    mut commands: Commands,
    radio_query: Query<(Entity, &CBRadio)>,
    emergency_channel_query: Query<&EmergencyChannel>,
    mut emergency_services: ResMut<EmergencyServicesAI>,
) {
    for (entity, radio) in radio_query.iter() {
        if let Ok(emergency_channel) = emergency_channel_query.get(entity) {
            if let Some(emergency_type) = &emergency_channel.active_emergency {
                // Process priority messages
                if emergency_channel.priority_level >= 2 {
                    // Handle high-priority emergency communications
                    emergency_services.handle_priority_message(entity, emergency_type);
                }
            }
        }
    }
}

pub fn update_emergency_scenarios(
    mut scenario_gen: ResMut<EmergencyScenarioGenerator>,
    time: Res<Time>,
    mut emergency_services: ResMut<EmergencyServicesAI>,
    world_state: Res<WorldState>,
) {
    // Generate new scenarios periodically
    if time.elapsed_seconds_f64() - scenario_gen.last_generation_time > 600.0 {
        if scenario_gen.active_scenarios.len() < 3 {
            let new_scenario = scenario_gen.generate_scenario(&world_state);
            scenario_gen.active_scenarios.push(new_scenario);
        }
        scenario_gen.last_generation_time = time.elapsed_seconds_f64();
    }

    // Update active scenarios
    scenario_gen.active_scenarios.retain_mut(|scenario| {
        // Check completion conditions for current stage
        if let Some(stage) = scenario.stages.get(scenario.current_stage) {
            let all_conditions_met = check_completion_conditions(stage, &world_state);
            
            if all_conditions_met {
                scenario.current_stage += 1;
                if scenario.current_stage >= scenario.stages.len() {
                    scenario.completion_time = Some(time.elapsed_seconds_f64());
                    return false; // Remove completed scenario
                }
            }
        }
        true
    });
}

fn check_completion_conditions(stage: &ScenarioStage, world_state: &WorldState) -> bool {
    for condition in &stage.completion_conditions {
        match condition {
            CompletionCondition::TimeElapsed(time_required) => {
                // Check if enough time has passed
                if world_state.current_time < *time_required {
                    return false;
                }
            }
            CompletionCondition::UnitsPresent(required_units) => {
                // Check if all required units are at the scene
                // This is a placeholder that should be implemented based on your unit tracking system
                return true;
            }
            CompletionCondition::HazardCleared(hazard_type) => {
                // Check if the specified hazard has been cleared
                // This should be implemented based on your hazard tracking system
                return true;
            }
            CompletionCondition::VictimsAssisted(count) => {
                // Check if the required number of victims have been assisted
                // This should be implemented based on your victim tracking system
                return true;
            }
            CompletionCondition::AreaSecured(location, radius) => {
                // Check if the area has been secured
                // This should be implemented based on your area security system
                return true;
            }
        }
    }
    true
}

#[derive(Resource)]
pub struct HazardTracker {
    active_hazards: HashMap<Entity, ActiveHazard>,
    cleared_hazards: Vec<ClearedHazard>,
}

#[derive(Clone, Debug)]
pub struct ActiveHazard {
    hazard_type: HazardType,
    location: Vec3,
    radius: f32,
    intensity: f32,
    start_time: f64,
    assigned_units: Vec<Entity>,
    containment_level: f32,
}

#[derive(Clone, Debug)]
pub struct ClearedHazard {
    hazard_type: HazardType,
    location: Vec3,
    clear_time: f64,
    clearing_units: Vec<Entity>,
    total_containment_time: f64,
}

impl HazardTracker {
    pub fn new() -> Self {
        Self {
            active_hazards: HashMap::new(),
            cleared_hazards: Vec::new(),
        }
    }

    pub fn add_hazard(&mut self, commands: &mut Commands, hazard_type: HazardType, location: Vec3, radius: f32, intensity: f32, time: f64) -> Entity {
        let hazard = ActiveHazard {
            hazard_type,
            location,
            radius,
            intensity,
            start_time: time,
            assigned_units: Vec::new(),
            containment_level: 0.0,
        };
        
        let entity = commands.spawn(()).id();
        self.active_hazards.insert(entity, hazard);
        entity
    }

    pub fn assign_unit(&mut self, hazard: Entity, unit: Entity) -> bool {
        if let Some(hazard_data) = self.active_hazards.get_mut(&hazard) {
            if !hazard_data.assigned_units.contains(&unit) {
                hazard_data.assigned_units.push(unit);
                return true;
            }
        }
        false
    }

    pub fn calculate_unit_effectiveness(&self, unit: &EmergencyUnitTracker, hazard: &ActiveHazard) -> f32 {
        let base_effectiveness = match (unit.unit_type, hazard.hazard_type) {
            (EmergencyUnitType::FireTruck, HazardType::Fire) => 2.0,
            (EmergencyUnitType::Ambulance, HazardType::ChemicalSpill) => 1.5,
            (EmergencyUnitType::RescueVehicle, HazardType::StructuralDamage) => 1.8,
            (EmergencyUnitType::PoliceCar, HazardType::RoadBlockage) => 1.6,
            _ => 1.0,
        };

        // Apply experience modifier
        let experience_modifier = 1.0 + (unit.experience_points as f32 * 0.01).min(0.5);
        
        // Calculate distance-based effectiveness using inverse square law
        let distance = (unit.current_location - hazard.location).length();
        let distance_factor = (1.0 / (1.0 + (distance / hazard.radius).powi(2))).clamp(0.1, 1.0);
        
        base_effectiveness * experience_modifier * distance_factor
    }

    pub fn update_containment(&mut self, hazard: Entity, unit: Entity, unit_tracker: &EmergencyUnitTracker, delta_time: f32) -> Option<f32> {
        if let Some(active_hazard) = self.active_hazards.get_mut(&hazard) {
            let effectiveness = self.calculate_unit_effectiveness(unit_tracker, active_hazard);
            
            // Calculate containment progress
            let containment_rate = effectiveness * delta_time * 0.1; // 10% base rate per second
            active_hazard.containment_level = (active_hazard.containment_level + containment_rate).min(1.0);
            
            // Reduce intensity based on containment
            active_hazard.intensity *= 1.0 - (containment_rate * 0.5);
            
            Some(active_hazard.containment_level)
        } else {
            None
        }
    }

    pub fn clear_hazard(&mut self, hazard: Entity, time: f64) -> bool {
        if let Some(active_hazard) = self.active_hazards.remove(&hazard) {
            let cleared = ClearedHazard {
                hazard_type: active_hazard.hazard_type,
                location: active_hazard.location,
                clear_time: time,
                clearing_units: active_hazard.assigned_units,
                total_containment_time: time - active_hazard.start_time,
            };
            self.cleared_hazards.push(cleared);
            true
        } else {
            false
        }
    }

    pub fn is_hazard_contained(&self, hazard: Entity) -> bool {
        self.active_hazards.get(&hazard)
            .map(|h| h.containment_level >= 95.0)
            .unwrap_or(false)
    }
}

// Update the check_completion_conditions function with proper implementations
fn check_completion_conditions(
    stage: &ScenarioStage,
    world_state: &WorldState,
    hazard_tracker: &HazardTracker,
    unit_query: &Query<(Entity, &EmergencyUnitTracker)>,
    victim_tracker: &VictimTracker,
    area_security: &AreaSecurity,
) -> bool {
    for condition in &stage.completion_conditions {
        match condition {
            CompletionCondition::TimeElapsed(time_required) => {
                if world_state.current_time < *time_required {
                    return false;
                }
            }
            CompletionCondition::UnitsPresent(required_units) => {
                let mut present_units = HashMap::new();
                for (_, unit) in unit_query.iter() {
                    if unit.current_status == UnitStatus::OnScene {
                        *present_units.entry(unit.unit_type).or_insert(0) += 1;
                    }
                }
                
                for &required_type in required_units {
                    if !present_units.contains_key(&required_type) {
                        return false;
                    }
                }
            }
            CompletionCondition::HazardCleared(hazard_type) => {
                let any_matching_active = hazard_tracker.active_hazards.values()
                    .any(|h| h.hazard_type == *hazard_type && h.containment_level < 95.0);
                if any_matching_active {
                    return false;
                }
            }
            CompletionCondition::VictimsAssisted(count) => {
                if victim_tracker.get_assisted_count() < *count {
                    return false;
                }
            }
            CompletionCondition::AreaSecured(location, radius) => {
                if !area_security.is_area_secured(*location, *radius) {
                    return false;
                }
            }
        }
    }
    true
}

// Add the system to update hazards
pub fn update_hazards(
    mut commands: Commands,
    time: Res<Time>,
    mut hazard_tracker: ResMut<HazardTracker>,
    unit_query: Query<(Entity, &EmergencyUnitTracker, &Transform)>,
) {
    let current_time = time.elapsed_seconds_f64();
    
    // Update containment levels based on nearby units
    for (hazard_entity, hazard) in hazard_tracker.active_hazards.iter() {
        let mut total_containment_effect = 0.0;
        
        for (unit_entity, unit, transform) in unit_query.iter() {
            if hazard.assigned_units.contains(&unit_entity) {
                let distance = transform.translation.distance(hazard.location);
                if distance <= hazard.radius {
                    // Units contribute more to containment when closer to the hazard
                    let distance_factor = 1.0 - (distance / hazard.radius).clamp(0.0, 1.0);
                    total_containment_effect += distance_factor * time.delta_seconds();
                }
            }
        }
        
        if total_containment_effect > 0.0 {
            hazard_tracker.update_containment(*hazard_entity, total_containment_effect);
            
            // Check if hazard is fully contained
            if hazard_tracker.is_hazard_contained(*hazard_entity) {
                hazard_tracker.clear_hazard(*hazard_entity, current_time);
            }
        }
    }
}

// Add the plugin registration
impl Plugin for EmergencyServicesPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EmergencyServicesAI>()
            .init_resource::<EmergencyScenarioGenerator>()
            .init_resource::<HazardTracker>()
            .add_systems(Update, (
                update_first_responder_roles,
                process_emergency_communications,
                update_emergency_units,
                process_dispatch_requests,
                update_emergency_status,
                update_emergency_scenarios,
                update_hazards,
            ));
    }
}

#[derive(Resource)]
pub struct VictimTracker {
    active_victims: HashMap<Entity, VictimStatus>,
    assisted_victims: Vec<AssistedVictim>,
}

#[derive(Clone, Debug)]
pub struct VictimStatus {
    location: Vec3,
    condition: VictimCondition,
    assigned_units: Vec<Entity>,
    assistance_level: f32,
    discovered_time: f64,
}

#[derive(Clone, Debug)]
pub struct AssistedVictim {
    location: Vec3,
    initial_condition: VictimCondition,
    assisting_units: Vec<Entity>,
    assistance_time: f64,
    total_assistance_time: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VictimCondition {
    Critical,
    Serious,
    Stable,
    Minor,
}

impl VictimTracker {
    pub fn new() -> Self {
        Self {
            active_victims: HashMap::new(),
            assisted_victims: Vec::new(),
        }
    }

    pub fn add_victim(&mut self, commands: &mut Commands, location: Vec3, condition: VictimCondition, time: f64) -> Entity {
        let victim = VictimStatus {
            location,
            condition,
            assigned_units: Vec::new(),
            assistance_level: 0.0,
            discovered_time: time,
        };
        
        let entity = commands.spawn(()).id();
        self.active_victims.insert(entity, victim);
        entity
    }

    pub fn assign_unit(&mut self, victim: Entity, unit: Entity) -> bool {
        if let Some(victim_data) = self.active_victims.get_mut(&victim) {
            if !victim_data.assigned_units.contains(&unit) {
                victim_data.assigned_units.push(unit);
                return true;
            }
        }
        false
    }

    pub fn calculate_assistance_effectiveness(&self, unit: &EmergencyUnitTracker, victim: &VictimStatus) -> f32 {
        let base_effectiveness = match (unit.unit_type, victim.condition) {
            (EmergencyUnitType::Ambulance, VictimCondition::Critical) => 2.0,
            (EmergencyUnitType::Ambulance, VictimCondition::Serious) => 1.8,
            (EmergencyUnitType::RescueVehicle, _) => 1.5,
            _ => 1.0,
        };

        // Apply experience and distance modifiers
        let experience_modifier = 1.0 + (unit.experience_points as f32 * 0.01).min(0.5);
        let distance = (unit.current_location - victim.location).length();
        let distance_factor = (1.0 / (1.0 + (distance / 10.0).powi(2))).clamp(0.1, 1.0);

        base_effectiveness * experience_modifier * distance_factor
    }

    pub fn update_assistance(&mut self, victim: Entity, unit: Entity, unit_tracker: &EmergencyUnitTracker, delta_time: f32) -> Option<f32> {
        if let Some(victim_status) = self.active_victims.get_mut(&victim) {
            let effectiveness = self.calculate_assistance_effectiveness(unit_tracker, victim_status);
            
            // Calculate assistance progress
            let assistance_rate = effectiveness * delta_time * 0.15; // 15% base rate per second
            victim_status.assistance_level = (victim_status.assistance_level + assistance_rate).min(1.0);
            
            // Update victim condition based on assistance level
            victim_status.condition = if victim_status.assistance_level >= 0.8 {
                VictimCondition::Minor
            } else if victim_status.assistance_level >= 0.6 {
                VictimCondition::Stable
            } else if victim_status.assistance_level >= 0.3 {
                VictimCondition::Serious
            } else {
                VictimCondition::Critical
            };
            
            Some(victim_status.assistance_level)
        } else {
            None
        }
    }

    pub fn mark_assisted(&mut self, victim: Entity, time: f64) -> bool {
        if let Some(active_victim) = self.active_victims.remove(&victim) {
            let assisted = AssistedVictim {
                location: active_victim.location,
                initial_condition: active_victim.condition,
                assisting_units: active_victim.assigned_units,
                assistance_time: time,
                total_assistance_time: time - active_victim.discovered_time,
            };
            self.assisted_victims.push(assisted);
            true
        } else {
            false
        }

    }

    pub fn get_assisted_count(&self) -> usize {
        self.assisted_victims.len()
    }

    pub fn is_victim_assisted(&self, victim: Entity) -> bool {
        self.active_victims.get(&victim)
            .map(|v| v.assistance_level >= 95.0)
            .unwrap_or(false)
    }

    pub fn get_critical_victims(&self) -> Vec<Entity> {
        self.active_victims
            .iter()
            .filter(|(_, status)| status.condition == VictimCondition::Critical)
            .map(|(&entity, _)| entity)
            .collect()
    }
}

#[derive(Resource)]
pub struct AreaSecurity {
    secured_zones: Vec<SecuredZone>,
    active_threats: HashMap<Entity, ThreatInfo>,
}

#[derive(Clone, Debug)]
pub struct SecuredZone {
    center: Vec3,
    radius: f32,
    securing_units: Vec<Entity>,
    security_level: f32,
}

#[derive(Clone, Debug)]
pub struct ThreatInfo {
    location: Vec3,
    threat_type: ThreatType,
    intensity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ThreatType {
    Hostile,
    Environmental,
    Structural,
    Chemical,
}

impl AreaSecurity {
    pub fn new() -> Self {
        Self {
            secured_zones: Vec::new(),
            active_threats: HashMap::new(),
        }
    }

    pub fn establish_zone(&mut self, center: Vec3, radius: f32) -> usize {
        let zone = SecuredZone {
            center,
            radius,
            securing_units: Vec::new(),
            security_level: 0.0,
        };
        self.secured_zones.push(zone);
        self.secured_zones.len() - 1
    }

    pub fn assign_unit_to_zone(&mut self, zone_index: usize, unit: Entity) -> bool {
        if let Some(zone) = self.secured_zones.get_mut(zone_index) {
            if !zone.securing_units.contains(&unit) {
                zone.securing_units.push(unit);
                return true;
            }
        }
        false
    }

    pub fn update_zone_security(&mut self, zone_index: usize, delta: f32) -> Option<f32> {
        if let Some(zone) = self.secured_zones.get_mut(zone_index) {
            zone.security_level = (zone.security_level + delta).clamp(0.0, 100.0);
            Some(zone.security_level)
        } else {
            None
        }
    }

    pub fn is_area_secured(&self, location: Vec3, radius: f32) -> bool {
        self.secured_zones.iter().any(|zone| {
            let distance = zone.center.distance(location);
            distance <= zone.radius + radius && zone.security_level >= 95.0
        })
    }

    pub fn add_threat(&mut self, commands: &mut Commands, location: Vec3, threat_type: ThreatType, intensity: f32) -> Entity {
        let threat = ThreatInfo {
            location,
            threat_type,
            intensity,
        };
        
        let entity = commands.spawn(()).id();
        self.active_threats.insert(entity, threat);
        entity
    }

    pub fn remove_threat(&mut self, threat: Entity) -> bool {
        self.active_threats.remove(&threat).is_some()
    }
}

// Add the system to update victims and area security
pub fn update_emergency_response(
    mut commands: Commands,
    time: Res<Time>,
    mut victim_tracker: ResMut<VictimTracker>,
    mut hazard_tracker: ResMut<HazardTracker>,
    mut area_security: ResMut<AreaSecurity>,
    unit_query: Query<(Entity, &EmergencyUnitTracker, &Transform)>,
) {
    let delta_time = time.delta_seconds();

    // Update hazard containment
    for (unit_entity, unit_tracker, _) in unit_query.iter() {
        if let Some(assigned_scenario) = unit_tracker.assigned_scenario {
            // Handle hazard containment
            for (&hazard_entity, hazard) in hazard_tracker.active_hazards.iter() {
                if hazard.assigned_units.contains(&unit_entity) {
                    hazard_tracker.update_containment(
                        hazard_entity,
                        unit_entity,
                        unit_tracker,
                        delta_time
                    );
                }
            }

            // Handle victim assistance
            for (&victim_entity, victim) in victim_tracker.active_victims.iter() {
                if victim.assigned_units.contains(&unit_entity) {
                    victim_tracker.update_assistance(
                        victim_entity,
                        unit_entity,
                        unit_tracker,
                        delta_time
                    );
                }
            }
        }
    }

    // Prioritize critical victims
    let critical_victims = victim_tracker.get_critical_victims();
    for victim_entity in critical_victims {
        if let Some(victim) = victim_tracker.active_victims.get(&victim_entity) {
            // Find nearest available medical unit
            if let Some((unit_entity, unit_tracker, _)) = unit_query
                .iter()
                .filter(|(_, unit, _)| unit.unit_type == EmergencyUnitType::Ambulance && unit.current_status == UnitStatus::Available)
                .min_by_key(|(_, _, transform)| {
                    let distance = (transform.translation - victim.location).length() as i32;
                    distance
                }) {
                // Assign unit to critical victim
                if victim_tracker.assign_unit(victim_entity, unit_entity) {
                    if let Some(mut unit_status) = commands.get_entity(unit_entity) {
                        unit_status.insert(UnitStatus::Responding);
                    }
                }
            }
        }
    }
}

// Update the plugin registration to include the new system
impl Plugin for EmergencyServicesPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<EmergencyServicesAI>()
            .init_resource::<EmergencyScenarioGenerator>()
            .init_resource::<HazardTracker>()
            .init_resource::<VictimTracker>()
            .init_resource::<AreaSecurity>()
            .add_systems(Update, (
                update_first_responder_roles,
                process_emergency_communications,
                update_emergency_units,
                process_dispatch_requests,
                update_emergency_status,
                update_emergency_scenarios,
                update_hazards,
                update_emergency_response,
            ));
    }
}