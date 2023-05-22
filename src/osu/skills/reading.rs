use crate::{Mods, osu::difficulty_object::OsuDifficultyObject};

use super::{Skill, previous};

#[derive(Clone, Debug)]
pub(crate) struct Reading {
    has_hidden_mod: bool,
    time_preempt: f64,
    time_fade_in: f64,
    difficulties: Vec<f64>,
}

impl Reading {
    const SKILL_MULTIPLIER: f64 = 2.4;

    pub(crate) fn new(mods: u32, time_preempt: f64, time_fade_in: f64) -> Self {
        Self {
            has_hidden_mod: mods.hd(),
            time_preempt,
            time_fade_in,
            difficulties: Vec::new(),
        }
    }
}

impl Skill for Reading {
    fn process(
        &mut self,
        curr: &OsuDifficultyObject<'_>,
        diff_objects: &[OsuDifficultyObject<'_>],
    ) {
        let reading_difficulty = ReadingEvaluator::evaluate_diff_of(curr, diff_objects, self.has_hidden_mod, self.time_preempt, self.time_fade_in) * Self::SKILL_MULTIPLIER;
        self.difficulties.push(reading_difficulty);
    }

    fn difficulty_value(&mut self) -> f64 {
        todo!()
    }
}

struct ReadingEvaluator;

impl ReadingEvaluator {
    const READING_WINDOW_SIZE: f64 = 3000.0;
    const FADE_OUT_DURATION_MULTIPLIER: f64 = 0.3;

    fn evaluate_diff_of(
        curr: &OsuDifficultyObject<'_>,
        diff_objects: &[OsuDifficultyObject<'_>],
        hidden: bool,
        time_preempt: f64,
        time_fade_in: f64,
    ) -> f64 {
        if curr.base.is_spinner() || curr.idx == 0 {
            return 0.0;
        }

        let curr_obj = curr;
        let curr_velocity = curr_obj.dists.lazy_jump_dist / curr_obj.strain_time;

        // TODO: pass actual clock rate?
        let clock_rate_estimate = curr_obj.base.start_time / curr_obj.strain_time;

        let mut past_object_difficulty_influence = 1.0;

        for i in (0..curr.idx).rev() {
            let loop_obj = &diff_objects[i];

            if curr_obj.start_time - loop_obj.start_time > Self::READING_WINDOW_SIZE || loop_obj.start_time < curr_obj.start_time - curr_obj.preempt {
                break;
            }

            let mut loop_difficulty = curr_obj.opacity_at(loop_obj.base.start_time, false, time_preempt, time_fade_in);

            // * Small distances means objects may be cheesed, so it doesn't matter whether they are arranged confusingly.
            loop_difficulty *= logistic((loop_obj.dists.min_jump_dist - 80.0) / 15.0);

            let time_between_curr_and_loop_obj = (curr_obj.base.start_time - loop_obj.base.start_time) / clock_rate_estimate;
            loop_difficulty *= Self::get_time_nerf_factor(time_between_curr_and_loop_obj);

            past_object_difficulty_influence += loop_difficulty;
        }

        let note_density_difficulty = (3.0 * (past_object_difficulty_influence - 1.0).max(1.0).ln()).powf(2.3);

        let mut hidden_difficulty = 0.0;

        if hidden {
            let time_spent_invisible = Self::get_duration_spent_invisible(curr_obj) / clock_rate_estimate;
            let time_difficulty_factor = 800.0 / past_object_difficulty_influence;

            hidden_difficulty += (7.0 * time_spent_invisible / time_difficulty_factor).powi(1) + 2.0 * curr_velocity;
        }

        let mut difficulty = hidden_difficulty + note_density_difficulty;
        difficulty *= Self::get_constant_angle_nerf_factor(curr_obj, diff_objects);

        difficulty
    }

    fn get_constant_angle_nerf_factor(current: &OsuDifficultyObject<'_>, diff_objects: &[OsuDifficultyObject<'_>]) -> f64 {
        const TIME_LIMIT: f64 = 2000.0;
        const TIME_LIMIT_LOW: f64 = 200.0;

        let mut constant_angle_count = 0.0;
        let mut index = 0;
        let mut current_time_gap = 0.0;

        while current_time_gap < TIME_LIMIT {  
            let loop_obj = previous(diff_objects, current.idx, index);
            if loop_obj.is_none() {
                break;
            }

            let loop_obj = loop_obj.unwrap();

            let long_interval_factor = f64::clamp(1.0 - (loop_obj.strain_time - TIME_LIMIT_LOW) / (TIME_LIMIT - TIME_LIMIT_LOW), 0.0, 1.1);
        
            if loop_obj.dists.angle.is_some() && current.dists.angle.is_some() {
                let loop_obj_angle = loop_obj.dists.angle.unwrap();
                let current_angle = current.dists.angle.unwrap();

                let angle_difference = (loop_obj_angle - current_angle).abs();
                constant_angle_count += (4.0 * (std::f64::consts::PI / 8.0).min(angle_difference)).cos() * long_interval_factor;
            }

            current_time_gap = current.start_time - loop_obj.start_time;
            index += 1;
        }

        (2.0 / constant_angle_count).min(1.0).powi(2)
    }

    fn get_duration_spent_invisible(curr: &OsuDifficultyObject<'_>) -> f64 {
        let base_object = curr.base;

        let fade_out_start_time = base_object.start_time - base_object.time_preempt + base_object.time_fade_in;
        let fade_out_duration = base_object.time_preempt * Self::FADE_OUT_DURATION_MULTIPLIER;

        (fade_out_start_time + fade_out_duration) - (base_object.start_time - base_object.time_preempt)
    }

    #[inline]
    fn get_time_nerf_factor(delta_time: f64) -> f64 {
        f64::clamp(2.0 - delta_time / (Self::READING_WINDOW_SIZE / 2.0), 0.0, 1.0)
    }
}

#[inline]
fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + std::f64::consts::E.powf(-x))
}
