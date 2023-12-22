use glam::Vec2;
use glam::Vec3;
pub trait Interactive<T: 'static = ()> {
    fn name_of_obj(&self);
    fn examine_obj(&mut self);
    fn use_obj(&mut self);
    fn push_obj(&mut self);
    fn pull_obj(&mut self);
    fn open_obj(&mut self);
    fn close_obj(&mut self);
    fn give_to_obj(&mut self);
    fn talk_to_obj(&mut self);
    fn take_obj(&mut self);
    fn get_3d_coords(&self) -> Vec3;
    fn get_2d_coords(&self) -> Vec2 {
        let td = self.get_3d_coords();
        Vec2 { x: td.x, y: td.y }
    }
}
