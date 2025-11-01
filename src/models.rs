use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct GraphQLRequest {
    pub query: String,
    pub variables: LoginVariables,
}

#[derive(Serialize)]
pub struct LoginVariables {
    pub u: String,
    pub p: String,
}

#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
pub struct LoginData {
    pub login: String,
}

#[derive(Deserialize)]
pub struct GetCalendarDaysData {
    pub get_calendar_days: Option<Vec<i64>>,
}

#[derive(Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

#[derive(Serialize)]
pub struct WorkoutRequest {
    pub query: String,
    pub variables: WorkoutVariables,
}

#[derive(Serialize)]
pub struct WorkoutVariables {
    pub uid: u32,
    pub ymd: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct WorkoutResponse {
    pub data: Option<WorkoutData>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
pub struct WorkoutData {
    pub jday: Option<JDay>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct JDay {
    pub log: String,
    pub bw: Option<f32>,
    pub eblocks: Vec<EBlock>,
    pub exercises: Vec<ExerciseWrapper>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct EBlock {
    pub eid: String,
    pub sets: Vec<Set>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Set {
    pub w: Option<f32>,
    pub r: Option<u32>,
    pub s: Option<u32>,
    pub lb: Option<f32>,
    pub rpe: Option<f32>,
    pub pr: Option<i32>,
    pub est1rm: Option<f32>,
    pub eff: Option<f32>,
    pub int: Option<f32>,
    #[serde(rename = "type")]
    pub set_type: Option<i32>,
    pub t: Option<f32>,
    pub d: Option<f32>,
    pub dunit: Option<String>,
    pub speed: Option<f32>,
    pub force: Option<f32>,
    pub c: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ExerciseWrapper {
    pub exercise: Exercise,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Exercise {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub ex_type: String,
}