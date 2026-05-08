use bevy::prelude::*;
use crate::game::vehicle::{Wheel, WheelBundle, Suspension};
use crate::game::plugins::weather::{Weather, WeatherState};

/// Test data for vehicle components
pub mod vehicle {
    use super::*;

    pub fn create_test_wheel() -> Wheel {
        Wheel {
            radius: 0.3,
            width: 0.2,
            mass: 20.0,
            inertia: 2.0,
            angular_velocity: 0.0,
            steering_angle: 0.0,
            drive_torque: 0.0,
            brake_torque: 0.0,
            ..Default::default()
        }
    }

    pub fn create_test_wheel_bundle() -> WheelBundle {
        WheelBundle {
            wheel: create_test_wheel(),
            transform: Transform::from_xyz(1.0, 0.3, 0.0),
            ..Default::default()
        }
    }

    pub fn create_test_suspension() -> Suspension {
        Suspension {
            spring_stiffness: 35000.0,
            damping: 4500.0,
            travel: 0.3,
            preload: 2000.0,
            anti_roll: 0.5,
            ..Default::default()
        }
    }
}

/// Test data for weather components
pub mod weather {
    use super::*;

    pub fn create_test_weather_state() -> WeatherState {
        WeatherState {
            current_weather: Weather::Clear,
            cloud_coverage: 0.0,
            precipitation: 0.0,
            wind_speed: 0.0,
            wind_direction: Vec2::new(1.0, 0.0),
            fog_density: 0.0,
            ..Default::default()
        }
    }

    pub fn create_rainy_weather_state() -> WeatherState {
        WeatherState {
            current_weather: Weather::Rain,
            cloud_coverage: 0.7,
            precipitation: 0.5,
            wind_speed: 5.0,
            wind_direction: Vec2::new(0.0, 1.0),
            fog_density: 0.2,
            ..Default::default()
        }
    }

    pub fn create_stormy_weather_state() -> WeatherState {
        WeatherState {
            current_weather: Weather::Storm,
            cloud_coverage: 1.0,
            precipitation: 1.0,
            wind_speed: 15.0,
            wind_direction: Vec2::new(-1.0, -1.0).normalize(),
            fog_density: 0.4,
            ..Default::default()
        }
    }
}

/// Test data for common transforms and vectors
pub mod transforms {
    use bevy::prelude::*;

    pub fn create_test_transform() -> Transform {
        Transform::from_xyz(0.0, 1.0, 0.0)
            .with_rotation(Quat::from_rotation_y(0.0))
            .with_scale(Vec3::ONE)
    }

    pub fn create_test_transform_with_rotation(rotation: f32) -> Transform {
        Transform::from_xyz(0.0, 1.0, 0.0)
            .with_rotation(Quat::from_rotation_y(rotation))
            .with_scale(Vec3::ONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel_creation() {
        let wheel = vehicle::create_test_wheel();
        assert_eq!(wheel.radius, 0.3);
        assert_eq!(wheel.width, 0.2);
        assert_eq!(wheel.mass, 20.0);
    }

    #[test]
    fn test_wheel_bundle_creation() {
        let bundle = vehicle::create_test_wheel_bundle();
        assert_eq!(bundle.transform.translation, Vec3::new(1.0, 0.3, 0.0));
    }

    #[test]
    fn test_suspension_creation() {
        let suspension = vehicle::create_test_suspension();
        assert_eq!(suspension.spring_stiffness, 35000.0);
        assert_eq!(suspension.damping, 4500.0);
        assert_eq!(suspension.travel, 0.3);
    }

    #[test]
    fn test_weather_state_creation() {
        let clear = weather::create_test_weather_state();
        assert_eq!(clear.current_weather, Weather::Clear);
        assert_eq!(clear.cloud_coverage, 0.0);

        let rainy = weather::create_rainy_weather_state();
        assert_eq!(rainy.current_weather, Weather::Rain);
        assert_eq!(rainy.precipitation, 0.5);

        let stormy = weather::create_stormy_weather_state();
        assert_eq!(stormy.current_weather, Weather::Storm);
        assert_eq!(stormy.wind_speed, 15.0);
    }

    #[test]
    fn test_transform_creation() {
        let transform = transforms::create_test_transform();
        assert_eq!(transform.translation, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(transform.scale, Vec3::ONE);

        let rotated = transforms::create_test_transform_with_rotation(std::f32::consts::PI);
        assert_eq!(rotated.translation, Vec3::new(0.0, 1.0, 0.0));
        assert!(rotated.rotation.abs_diff_eq(
            Quat::from_rotation_y(std::f32::consts::PI),
            0.0001
        ));
    }
} 