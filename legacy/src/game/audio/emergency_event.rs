use bevy::prelude::*;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Represents the severity level of an emergency event
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SeverityLevel {
    Low,      // Minor incidents, no immediate danger
    Medium,   // Significant incidents requiring attention
    High,     // Serious incidents requiring immediate response
    Critical, // Life-threatening situations
}

impl SeverityLevel {
    pub fn to_float(&self) -> f32 {
        match self {
            SeverityLevel::Low => 0.25,
            SeverityLevel::Medium => 0.5,
            SeverityLevel::High => 0.75,
            SeverityLevel::Critical => 1.0,
        }
    }

    pub fn from_float(value: f32) -> Self {
        match value {
            v if v < 0.3 => SeverityLevel::Low,
            v if v < 0.6 => SeverityLevel::Medium,
            v if v < 0.8 => SeverityLevel::High,
            _ => SeverityLevel::Critical,
        }
    }
}

/// Detailed emergency event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyEvent {
    pub id: String,
    pub event_type: EmergencyType,
    pub severity: SeverityLevel,
    pub location: Vec3,
    pub start_time: f64,
    pub estimated_duration: Duration,
    pub status: EmergencyStatus,
    pub details: EmergencyDetails,
    pub required_response: Vec<EmergencyUnitType>,
    pub responding_units: Vec<String>,
    pub updates: Vec<EmergencyUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyDetails {
    pub casualties: Option<u32>,
    pub vehicles_involved: Option<u32>,
    pub hazmat_present: bool,
    pub fire_present: bool,
    pub trapped_victims: bool,
    pub weather_conditions: Option<String>,
    pub road_conditions: Option<String>,
    pub additional_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyUpdate {
    pub timestamp: f64,
    pub update_type: UpdateType,
    pub message: String,
    pub reporter: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateType {
    Initial,
    StatusChange,
    UnitArrival,
    UnitDeparture,
    SituationUpdate,
    ResourceRequest,
    Resolution,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmergencyStatus {
    Reported,
    UnitsDispatched,
    UnitsOnScene,
    InProgress,
    Contained,
    Resolved,
    Cancelled,
}

/// Emergency communication protocols
#[derive(Debug, Clone)]
pub struct EmergencyProtocol {
    pub channel: u8,
    pub priority_level: u8,
    pub broadcast_range: f32,
    pub repeat_interval: Duration,
}

impl Default for EmergencyProtocol {
    fn default() -> Self {
        Self {
            channel: 9,  // Standard emergency channel
            priority_level: 1,
            broadcast_range: 50000.0, // 50km range
            repeat_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl EmergencyEvent {
    pub fn new(
        event_type: EmergencyType,
        severity: SeverityLevel,
        location: Vec3,
        details: EmergencyDetails,
    ) -> Self {
        let id = format!("EM-{}-{}", chrono::Utc::now().timestamp(), fastrand::u32(..));
        let start_time = chrono::Utc::now().timestamp() as f64;
        
        Self {
            id,
            event_type,
            severity,
            location,
            start_time,
            estimated_duration: Self::calculate_estimated_duration(&event_type, severity),
            status: EmergencyStatus::Reported,
            details,
            required_response: Self::determine_required_units(&event_type, severity),
            responding_units: Vec::new(),
            updates: vec![EmergencyUpdate {
                timestamp: start_time,
                update_type: UpdateType::Initial,
                message: format!("Emergency reported: {} - {}", event_type.to_string(), severity.to_string()),
                reporter: "SYSTEM".to_string(),
            }],
        }
    }

    fn calculate_estimated_duration(event_type: &EmergencyType, severity: SeverityLevel) -> Duration {
        let base_duration = match event_type {
            EmergencyType::Accident => 60,
            EmergencyType::MedicalEmergency => 30,
            EmergencyType::VehicleFire => 45,
            EmergencyType::HazardousMaterials => 180,
            EmergencyType::WeatherHazard => 120,
            EmergencyType::RoadHazard => 90,
            EmergencyType::VehicleBreakdown => 45,
            EmergencyType::LawEnforcement => 60,
            EmergencyType::Search => 240,
            EmergencyType::Other => 60,
        };

        let severity_multiplier = match severity {
            SeverityLevel::Low => 0.5,
            SeverityLevel::Medium => 1.0,
            SeverityLevel::High => 2.0,
            SeverityLevel::Critical => 3.0,
        };

        Duration::from_mins((base_duration as f32 * severity_multiplier) as u64)
    }

    fn determine_required_units(event_type: &EmergencyType, severity: SeverityLevel) -> Vec<EmergencyUnitType> {
        let mut units = Vec::new();
        
        // Base units based on event type
        match event_type {
            EmergencyType::Accident => {
                units.push(EmergencyUnitType::Police);
                units.push(EmergencyUnitType::Ambulance);
                units.push(EmergencyUnitType::TowTruck);
            },
            EmergencyType::MedicalEmergency => {
                units.push(EmergencyUnitType::Ambulance);
                if severity >= SeverityLevel::High {
                    units.push(EmergencyUnitType::Police);
                }
            },
            EmergencyType::VehicleFire => {
                units.push(EmergencyUnitType::FireTruck);
                units.push(EmergencyUnitType::Police);
                if severity >= SeverityLevel::High {
                    units.push(EmergencyUnitType::Ambulance);
                }
            },
            EmergencyType::HazardousMaterials => {
                units.push(EmergencyUnitType::HazmatTeam);
                units.push(EmergencyUnitType::FireTruck);
                units.push(EmergencyUnitType::Police);
                if severity >= SeverityLevel::High {
                    units.push(EmergencyUnitType::Ambulance);
                }
            },
            EmergencyType::WeatherHazard | EmergencyType::RoadHazard => {
                units.push(EmergencyUnitType::Police);
                if severity >= SeverityLevel::High {
                    units.push(EmergencyUnitType::FireTruck);
                }
            },
            EmergencyType::VehicleBreakdown => {
                units.push(EmergencyUnitType::TowTruck);
                if severity >= SeverityLevel::Medium {
                    units.push(EmergencyUnitType::Police);
                }
            },
            EmergencyType::LawEnforcement => {
                units.push(EmergencyUnitType::Police);
                if severity >= SeverityLevel::High {
                    units.push(EmergencyUnitType::Ambulance);
                }
            },
            EmergencyType::Search => {
                units.push(EmergencyUnitType::Police);
                if severity >= SeverityLevel::Medium {
                    units.push(EmergencyUnitType::Ambulance);
                }
            },
            EmergencyType::Other => {
                units.push(EmergencyUnitType::Police);
            },
        }

        // Add additional units for critical situations
        if severity == SeverityLevel::Critical {
            if !units.contains(&EmergencyUnitType::Ambulance) {
                units.push(EmergencyUnitType::Ambulance);
            }
            if !units.contains(&EmergencyUnitType::Police) {
                units.push(EmergencyUnitType::Police);
            }
        }

        units
    }

    pub fn add_update(&mut self, update_type: UpdateType, message: String, reporter: String) {
        let update = EmergencyUpdate {
            timestamp: chrono::Utc::now().timestamp() as f64,
            update_type,
            message,
            reporter,
        };
        self.updates.push(update);
    }

    pub fn update_status(&mut self, new_status: EmergencyStatus, reporter: String) {
        self.status = new_status.clone();
        self.add_update(
            UpdateType::StatusChange,
            format!("Status updated to: {:?}", new_status),
            reporter,
        );
    }

    pub fn add_responding_unit(&mut self, unit_id: String) {
        if !self.responding_units.contains(&unit_id) {
            self.responding_units.push(unit_id.clone());
            self.add_update(
                UpdateType::UnitArrival,
                format!("Unit {} assigned to incident", unit_id),
                "DISPATCH".to_string(),
            );
        }
    }

    pub fn remove_responding_unit(&mut self, unit_id: &str) {
        if let Some(pos) = self.responding_units.iter().position(|x| x == unit_id) {
            self.responding_units.remove(pos);
            self.add_update(
                UpdateType::UnitDeparture,
                format!("Unit {} cleared from incident", unit_id),
                "DISPATCH".to_string(),
            );
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self.status, EmergencyStatus::Resolved | EmergencyStatus::Cancelled)
    }

    pub fn get_priority_level(&self) -> u8 {
        match self.severity {
            SeverityLevel::Critical => 1,
            SeverityLevel::High => 2,
            SeverityLevel::Medium => 3,
            SeverityLevel::Low => 4,
        }
    }

    pub fn get_communication_protocol(&self) -> EmergencyProtocol {
        let mut protocol = EmergencyProtocol::default();
        
        // Adjust protocol based on severity
        protocol.priority_level = self.get_priority_level();
        
        // Adjust repeat interval based on severity
        protocol.repeat_interval = match self.severity {
            SeverityLevel::Critical => Duration::from_secs(60),    // Every 1 minute
            SeverityLevel::High => Duration::from_secs(120),       // Every 2 minutes
            SeverityLevel::Medium => Duration::from_secs(300),     // Every 5 minutes
            SeverityLevel::Low => Duration::from_secs(600),        // Every 10 minutes
        };

        protocol
    }
}

/// Resource for managing active emergency events
#[derive(Resource)]
pub struct EmergencyEventManager {
    active_events: Vec<EmergencyEvent>,
    event_history: Vec<EmergencyEvent>,
    next_update: f64,
    update_interval: Duration,
}

impl Default for EmergencyEventManager {
    fn default() -> Self {
        Self {
            active_events: Vec::new(),
            event_history: Vec::new(),
            next_update: 0.0,
            update_interval: Duration::from_secs(1),
        }
    }
}

impl EmergencyEventManager {
    pub fn add_event(&mut self, event: EmergencyEvent) {
        self.active_events.push(event);
    }

    pub fn get_active_events(&self) -> &[EmergencyEvent] {
        &self.active_events
    }

    pub fn get_event_by_id(&self, id: &str) -> Option<&EmergencyEvent> {
        self.active_events.iter().find(|e| e.id == id)
    }

    pub fn get_event_by_id_mut(&mut self, id: &str) -> Option<&mut EmergencyEvent> {
        self.active_events.iter_mut().find(|e| e.id == id)
    }

    pub fn update(&mut self, current_time: f64) {
        if current_time < self.next_update {
            return;
        }

        // Move resolved events to history
        let resolved: Vec<_> = self.active_events
            .iter()
            .filter(|e| e.is_resolved())
            .cloned()
            .collect();

        for event in resolved {
            if let Some(pos) = self.active_events.iter().position(|e| e.id == event.id) {
                let event = self.active_events.remove(pos);
                self.event_history.push(event);
            }
        }

        self.next_update = current_time + self.update_interval.as_secs_f64();
    }
}

pub struct EmergencyEventPlugin;

impl Plugin for EmergencyEventPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EmergencyEventManager>()
           .add_systems(Update, update_emergency_events);
    }
}

fn update_emergency_events(
    mut event_manager: ResMut<EmergencyEventManager>,
    time: Res<Time>,
) {
    event_manager.update(time.elapsed_seconds_f64());
} 