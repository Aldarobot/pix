// el.rs        Pixel format.
//
// Copyright (c) 2018-2020  Douglas P Lau
// Copyright (c) 2019-2020  Jeron Aldaron Lau
//
//! Module for `pix::el` items
use crate::chan::{Alpha, Channel, Gamma, Premultiplied};
use crate::matte::Matte;
use crate::ops::PorterDuff;
use crate::private::Sealed;
use crate::rgb::Rgb;
use crate::ColorModel;
use std::any::TypeId;
use std::fmt::Debug;
use std::marker::PhantomData;

/// Pixel [channel], [color model], [alpha] and [gamma] mode.
///
/// A pixel can be converted to another format using the [convert] method.
///
/// [alpha]: ../chan/trait.Alpha.html
/// [channel]: ../chan/trait.Channel.html
/// [color model]: ../trait.ColorModel.html
/// [convert]: #method.convert
/// [gamma]: ../chan/trait.Gamma.html
///
/// ### Type Alias Naming Scheme
///
/// * _Gamma_: `S` for [sRGB] gamma encoding; [linear] if omitted.
/// * _Color model_: [`Rgb`] / [`Bgr`] / [`Gray`] / [`Hsv`] / [`Hsl`] /
///                  [`Hwb`] / [`YCbCr`] / [`Matte`].
/// * _Alpha_: `a` to include alpha channel enabling translucent pixels.
/// * _Bit depth_: `8` / `16` / `32` for 8-bit integer, 16-bit integer and
///   32-bit floating-point [channels].
/// * _Alpha mode_: `p` for [premultiplied]; [straight] if omitted.
///
/// [`bgr`]: ../bgr/struct.Bgr.html
/// [channels]: ../chan/trait.Channel.html
/// [`gray`]: ../gray/struct.Gray.html
/// [`hsl`]: ../hsl/struct.Hsl.html
/// [`hsv`]: ../hsv/struct.Hsv.html
/// [`hwb`]: ../hwb/struct.Hwb.html
/// [linear]: ../chan/struct.Linear.html
/// [`matte`]: ../matte/struct.Matte.html
/// [premultiplied]: ../chan/struct.Premultiplied.html
/// [`Rgb`]: ../rgb/struct.Rgb.html
/// [sRGB]: ../chan/struct.Srgb.html
/// [straight]: ../chan/struct.Straight.html
/// [`YCbCr`]: ../ycc/struct.YCbCr.html
///
/// This trait is *sealed*, and cannot be implemented outside of this crate.
pub trait Pixel: Clone + Copy + Debug + Default + PartialEq + Sealed {
    /// Channel type
    type Chan: Channel;

    /// Color model
    type Model: ColorModel;

    /// Alpha mode
    type Alpha: Alpha;

    /// Gamma mode
    type Gamma: Gamma;

    /// Make a pixel from a slice of channels.
    fn from_channels(ch: &[Self::Chan]) -> Self;

    /// Convert from a pixel with a different bit depth.
    fn from_bit_depth<P>(p: P) -> Self
    where
        P: Pixel,
        Self::Chan: From<P::Chan>;

    /// Get the channels.
    fn channels(&self) -> &[Self::Chan];

    /// Get the channels mutably.
    fn channels_mut(&mut self) -> &mut [Self::Chan];

    /// Get the first channel.
    fn one(self) -> Self::Chan {
        *self.channels().get(0).unwrap_or(&Self::Chan::MAX)
    }

    /// Get a mutable reference to the first channel
    fn one_mut(&mut self) -> &mut Self::Chan {
        &mut self.channels_mut()[0]
    }

    /// Get the second channel.
    fn two(self) -> Self::Chan {
        *self.channels().get(1).unwrap_or(&Self::Chan::MAX)
    }

    /// Get a mutable reference to the second channel
    fn two_mut(&mut self) -> &mut Self::Chan {
        &mut self.channels_mut()[1]
    }

    /// Get the third channel.
    fn three(self) -> Self::Chan {
        *self.channels().get(2).unwrap_or(&Self::Chan::MAX)
    }

    /// Get a mutable reference to the third channel
    fn three_mut(&mut self) -> &mut Self::Chan {
        &mut self.channels_mut()[2]
    }

    /// Get the fourth channel.
    fn four(self) -> Self::Chan {
        *self.channels().get(3).unwrap_or(&Self::Chan::MAX)
    }

    /// Get a mutable reference to the fourth channel
    fn four_mut(&mut self) -> &mut Self::Chan {
        &mut self.channels_mut()[3]
    }

    /// Get the *alpha* channel.
    ///
    /// # Example: Get Alpha
    /// ```
    /// use pix::chan::Ch16;
    /// use pix::el::Pixel;
    /// use pix::gray::Graya16;
    ///
    /// let p = Graya16::new(0x7090, 0x6010);
    /// assert_eq!(Pixel::alpha(p), Ch16::new(0x6010));
    /// ```
    fn alpha(self) -> Self::Chan {
        let chan = self.channels();
        *chan.get(Self::Model::ALPHA).unwrap_or(&Self::Chan::MAX)
    }

    /// Get a mutable reference to the *alpha* channel.
    ///
    /// # Panics
    ///
    /// Panics if the pixel does not contain an alpha channel.
    ///
    /// # Example: Set Alpha
    /// ```
    /// use pix::chan::Ch8;
    /// use pix::el::Pixel;
    /// use pix::rgb::Rgba8;
    ///
    /// let mut p = Rgba8::new(0xFF, 0x40, 0x80, 0xA5);
    /// *Pixel::alpha_mut(&mut p) = Ch8::new(0x4B);
    /// assert_eq!(Pixel::alpha(p), Ch8::new(0x4B));
    /// ```
    fn alpha_mut(&mut self) -> &mut Self::Chan {
        let chan = self.channels_mut();
        chan.get_mut(Self::Model::ALPHA).unwrap()
    }

    /// Convert a pixel to another format
    ///
    /// * `D` Destination format.
    fn convert<D>(self) -> D
    where
        D: Pixel,
        D::Chan: From<Self::Chan>,
    {
        if TypeId::of::<Self::Model>() == TypeId::of::<D::Model>() {
            convert_same_model::<D, Self>(self)
        } else {
            convert_thru_rgba::<D, Self>(self)
        }
    }

    /// Copy a color to a pixel slice
    fn copy_color(dst: &mut [Self], clr: &Self) {
        for d in dst.iter_mut() {
            *d = *clr;
        }
    }

    /// Copy a slice to another
    fn copy_slice(dst: &mut [Self], src: &[Self]) {
        for (d, s) in dst.iter_mut().zip(src) {
            *d = *s;
        }
    }

    /// Composite a color with a pixel slice
    fn composite_color<O>(dst: &mut [Self], clr: &Self, op: O)
    where
        Self: Pixel<Alpha = Premultiplied>,
        O: PorterDuff,
    {
        for d in dst.iter_mut() {
            d.composite_channels(clr, op);
        }
    }

    /// Composite matte with color to destination pixel slice
    fn composite_matte<M, O>(dst: &mut [Self], src: &[M], clr: &Self, op: O)
    where
        Self: Pixel<Alpha = Premultiplied>,
        M: Pixel<Chan = Self::Chan, Model = Matte, Gamma = Self::Gamma>,
        O: PorterDuff,
    {
        for (d, s) in dst.iter_mut().zip(src) {
            d.composite_channels_matte(&s.alpha(), clr, op);
        }
    }

    /// Composite two slices of pixels
    fn composite_slice<O>(dst: &mut [Self], src: &[Self], op: O)
    where
        Self: Pixel<Alpha = Premultiplied>,
        O: PorterDuff,
    {
        for (d, s) in dst.iter_mut().zip(src) {
            d.composite_channels(s, op);
        }
    }

    /// Composite the channels of two pixels
    fn composite_channels<O>(&mut self, src: &Self, _op: O)
    where
        Self: Pixel<Alpha = Premultiplied>,
        O: PorterDuff,
    {
        // FIXME: composite circular channels
        let da1 = Self::Chan::MAX - self.alpha();
        let sa1 = Self::Chan::MAX - src.alpha();
        let d_chan = &mut self.channels_mut()[Self::Model::LINEAR];
        let s_chan = &src.channels()[Self::Model::LINEAR];
        d_chan
            .iter_mut()
            .zip(s_chan)
            .for_each(|(d, s)| O::composite(d, da1, s, sa1));
        O::composite(self.alpha_mut(), da1, &src.alpha(), sa1);
    }

    /// Composite the channels of pixels with a matte and color
    fn composite_channels_matte<O>(
        &mut self,
        alpha: &Self::Chan,
        src: &Self,
        _op: O,
    ) where
        Self: Pixel<Alpha = Premultiplied>,
        O: PorterDuff,
    {
        // FIXME: composite circular channels
        let da1 = Self::Chan::MAX - self.alpha();
        let sa1 = Self::Chan::MAX - *alpha;
        let d_chan = &mut self.channels_mut()[Self::Model::LINEAR];
        let s_chan = &src.channels()[Self::Model::LINEAR];
        d_chan
            .iter_mut()
            .zip(s_chan)
            .for_each(|(d, s)| O::composite(d, da1, &(*s * *alpha), sa1));
        O::composite(self.alpha_mut(), da1, &(src.alpha() * *alpha), sa1);
    }
}

/// Rgba pixel type for color model conversions
pub type PixRgba<P> =
    Pix4<<P as Pixel>::Chan, Rgb, <P as Pixel>::Alpha, <P as Pixel>::Gamma>;

/// Convert a pixel to another format with the same color model.
///
/// * `D` Destination pixel format.
/// * `S` Source pixel format.
/// * `src` Source pixel.
fn convert_same_model<D, S>(src: S) -> D
where
    D: Pixel,
    S: Pixel,
    D::Chan: From<S::Chan>,
{
    let mut dst = D::from_bit_depth(src);
    if TypeId::of::<S::Alpha>() != TypeId::of::<D::Alpha>()
        || TypeId::of::<S::Gamma>() != TypeId::of::<D::Gamma>()
    {
        let alpha = dst.alpha();
        let mut channels = dst.channels_mut();
        convert_alpha_gamma::<D, S>(&mut channels, alpha);
    }
    dst
}

/// Convert *alpha* / *gamma* to another pixel format
fn convert_alpha_gamma<D, S>(channels: &mut [D::Chan], alpha: D::Chan)
where
    D: Pixel,
    S: Pixel,
{
    for c in channels[D::Model::LINEAR].iter_mut() {
        *c = S::Gamma::to_linear(*c);
        if TypeId::of::<S::Alpha>() != TypeId::of::<D::Alpha>() {
            *c = S::Alpha::decode(*c, alpha);
            *c = D::Alpha::encode(*c, alpha);
        }
        *c = D::Gamma::from_linear(*c);
    }
}

/// Convert a pixel to another format thru RGBA.
///
/// * `D` Destination pixel format.
/// * `S` Source pixel format.
/// * `src` Source pixel.
fn convert_thru_rgba<D, S>(src: S) -> D
where
    D: Pixel,
    S: Pixel,
    D::Chan: From<S::Chan>,
{
    let rgba = S::Model::into_rgba::<S>(src);
    let rgba = convert_same_model::<PixRgba<D>, PixRgba<S>>(rgba);
    D::Model::from_rgba::<D>(rgba)
}

/// [Pixel] with one [channel] in its [color model].
///
/// [channel]: ../chan/trait.Channel.html
/// [color model]: ../trait.ColorModel.html
/// [pixel]: trait.Pixel.html
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Pix1<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    channels: [C; 1],
    _model: PhantomData<M>,
    _alpha: PhantomData<A>,
    _gamma: PhantomData<G>,
}

impl<C, M, A, G> Pix1<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    /// Create a one-channel color.
    ///
    /// ## Example
    /// ```
    /// use pix::gray::Gray8;
    ///
    /// let opaque_gray = Gray8::new(128);
    /// ```
    pub fn new<H>(one: H) -> Self
    where
        C: From<H>,
    {
        let channels = [C::from(one); 1];
        Pix1 {
            channels,
            _model: PhantomData,
            _alpha: PhantomData,
            _gamma: PhantomData,
        }
    }
}

impl<C, M, A, G> Pixel for Pix1<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    type Chan = C;
    type Model = M;
    type Alpha = A;
    type Gamma = G;

    fn from_channels(ch: &[C]) -> Self {
        let one = ch[0].into();
        Self::new(one)
    }

    fn from_bit_depth<P>(p: P) -> Self
    where
        P: Pixel,
        Self::Chan: From<P::Chan>,
    {
        if TypeId::of::<Self::Model>() != TypeId::of::<P::Model>() {
            panic!("Invalid pixel conversion");
        }
        let one = Self::Chan::from(p.one());
        Self::new(one)
    }

    fn channels(&self) -> &[Self::Chan] {
        &self.channels
    }

    fn channels_mut(&mut self) -> &mut [Self::Chan] {
        &mut self.channels
    }
}

/// [Pixel] with two [channel]s in its [color model].
///
/// [channel]: ../chan/trait.Channel.html
/// [color model]: ../trait.ColorModel.html
/// [pixel]: trait.Pixel.html
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Pix2<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    channels: [C; 2],
    _model: PhantomData<M>,
    _alpha: PhantomData<A>,
    _gamma: PhantomData<G>,
}

impl<C, M, A, G> Pix2<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    /// Create a two-channel color.
    ///
    /// ## Example
    /// ```
    /// use pix::gray::Graya8;
    ///
    /// let translucent_gray = Graya8::new(128, 200);
    /// ```
    pub fn new<H>(one: H, two: H) -> Self
    where
        C: From<H>,
    {
        let one = C::from(one);
        let two = C::from(two);
        let channels = [one, two];
        Pix2 {
            channels,
            _model: PhantomData,
            _alpha: PhantomData,
            _gamma: PhantomData,
        }
    }
}

impl<C, M, A, G> Pixel for Pix2<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    type Chan = C;
    type Model = M;
    type Alpha = A;
    type Gamma = G;

    fn from_channels(ch: &[C]) -> Self {
        let one = ch[0].into();
        let two = ch[1].into();
        Self::new(one, two)
    }

    fn from_bit_depth<P>(p: P) -> Self
    where
        P: Pixel,
        Self::Chan: From<P::Chan>,
    {
        if TypeId::of::<Self::Model>() != TypeId::of::<P::Model>() {
            panic!("Invalid pixel conversion");
        }
        let one = Self::Chan::from(p.one());
        let two = Self::Chan::from(p.two());
        Self::new(one, two)
    }

    fn channels(&self) -> &[Self::Chan] {
        &self.channels
    }

    fn channels_mut(&mut self) -> &mut [Self::Chan] {
        &mut self.channels
    }
}

/// [Pixel] with three [channel]s in its [color model].
///
/// [channel]: ../chan/trait.Channel.html
/// [color model]: ../trait.ColorModel.html
/// [pixel]: trait.Pixel.html
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Pix3<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    channels: [C; 3],
    _model: PhantomData<M>,
    _alpha: PhantomData<A>,
    _gamma: PhantomData<G>,
}

impl<C, M, A, G> Pix3<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    /// Create a three-channel color.
    ///
    /// ## Example
    /// ```
    /// use pix::rgb::Rgb8;
    ///
    /// let rgb = Rgb8::new(128, 200, 255);
    /// ```
    pub fn new<H>(one: H, two: H, three: H) -> Self
    where
        C: From<H>,
    {
        let one = C::from(one);
        let two = C::from(two);
        let three = C::from(three);
        let channels = [one, two, three];
        Pix3 {
            channels,
            _model: PhantomData,
            _alpha: PhantomData,
            _gamma: PhantomData,
        }
    }
}

impl<C, M, A, G> Pixel for Pix3<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    type Chan = C;
    type Model = M;
    type Alpha = A;
    type Gamma = G;

    fn from_channels(ch: &[C]) -> Self {
        let one = ch[0].into();
        let two = ch[1].into();
        let three = ch[2].into();
        Self::new(one, two, three)
    }

    fn from_bit_depth<P>(p: P) -> Self
    where
        P: Pixel,
        Self::Chan: From<P::Chan>,
    {
        if TypeId::of::<Self::Model>() != TypeId::of::<P::Model>() {
            panic!("Invalid pixel conversion");
        }
        let one = Self::Chan::from(p.one());
        let two = Self::Chan::from(p.two());
        let three = Self::Chan::from(p.three());
        Self::new(one, two, three)
    }

    fn channels(&self) -> &[Self::Chan] {
        &self.channels
    }

    fn channels_mut(&mut self) -> &mut [Self::Chan] {
        &mut self.channels
    }
}

/// [Pixel] with four [channel]s in its [color model].
///
/// [channel]: ../chan/trait.Channel.html
/// [color model]: ../trait.ColorModel.html
/// [pixel]: trait.Pixel.html
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Pix4<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    channels: [C; 4],
    _model: PhantomData<M>,
    _alpha: PhantomData<A>,
    _gamma: PhantomData<G>,
}

impl<C, M, A, G> Pix4<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    /// Create a four-channel color.
    ///
    /// ## Example
    /// ```
    /// use pix::rgb::Rgba8;
    ///
    /// let rgba = Rgba8::new(128, 200, 255, 128);
    /// ```
    pub fn new<H>(one: H, two: H, three: H, four: H) -> Self
    where
        C: From<H>,
    {
        let one = C::from(one);
        let two = C::from(two);
        let three = C::from(three);
        let four = C::from(four);
        let channels = [one, two, three, four];
        Pix4 {
            channels,
            _model: PhantomData,
            _alpha: PhantomData,
            _gamma: PhantomData,
        }
    }
}

impl<C, M, A, G> Pixel for Pix4<C, M, A, G>
where
    C: Channel,
    M: ColorModel,
    A: Alpha,
    G: Gamma,
{
    type Chan = C;
    type Model = M;
    type Alpha = A;
    type Gamma = G;

    fn from_channels(ch: &[C]) -> Self {
        let one = ch[0].into();
        let two = ch[1].into();
        let three = ch[2].into();
        let four = ch[3].into();
        Self::new(one, two, three, four)
    }

    fn from_bit_depth<P>(p: P) -> Self
    where
        P: Pixel,
        Self::Chan: From<P::Chan>,
    {
        if TypeId::of::<Self::Model>() != TypeId::of::<P::Model>() {
            panic!("Invalid pixel conversion");
        }
        let one = Self::Chan::from(p.one());
        let two = Self::Chan::from(p.two());
        let three = Self::Chan::from(p.three());
        let four = Self::Chan::from(p.four());
        Self::new(one, two, three, four)
    }

    fn channels(&self) -> &[Self::Chan] {
        &self.channels
    }

    fn channels_mut(&mut self) -> &mut [Self::Chan] {
        &mut self.channels
    }
}

#[cfg(test)]
mod test {
    use crate::el::*;
    use crate::gray::*;
    use crate::matte::*;
    use crate::rgb::*;

    #[test]
    fn check_sizes() {
        assert_eq!(std::mem::size_of::<Matte8>(), 1);
        assert_eq!(std::mem::size_of::<Matte16>(), 2);
        assert_eq!(std::mem::size_of::<Matte32>(), 4);
        assert_eq!(std::mem::size_of::<SGray8>(), 1);
        assert_eq!(std::mem::size_of::<SGray16>(), 2);
        assert_eq!(std::mem::size_of::<SGray32>(), 4);
        assert_eq!(std::mem::size_of::<SGraya8>(), 2);
        assert_eq!(std::mem::size_of::<SGraya16>(), 4);
        assert_eq!(std::mem::size_of::<SGraya32>(), 8);
        assert_eq!(std::mem::size_of::<Rgb8>(), 3);
        assert_eq!(std::mem::size_of::<Rgb16>(), 6);
        assert_eq!(std::mem::size_of::<Rgb32>(), 12);
        assert_eq!(std::mem::size_of::<Rgba8>(), 4);
        assert_eq!(std::mem::size_of::<Rgba16>(), 8);
        assert_eq!(std::mem::size_of::<Rgba32>(), 16);
    }

    #[test]
    fn gray_to_rgb() {
        assert_eq!(SRgb8::new(0xD9, 0xD9, 0xD9), SGray8::new(0xD9).convert(),);
        assert_eq!(
            SRgb8::new(0x33, 0x33, 0x33),
            SGray16::new(0x337F).convert(),
        );
        assert_eq!(SRgb8::new(0x40, 0x40, 0x40), SGray32::new(0.25).convert(),);
        assert_eq!(
            SRgb16::new(0x2929, 0x2929, 0x2929),
            SGray8::new(0x29).convert(),
        );
        assert_eq!(
            SRgb16::new(0x5593, 0x5593, 0x5593),
            SGray16::new(0x5593).convert(),
        );
        assert_eq!(
            SRgb16::new(0xFFFF, 0xFFFF, 0xFFFF),
            SGray32::new(1.0).convert(),
        );
        assert_eq!(
            SRgb32::new(0.5019608, 0.5019608, 0.5019608),
            SGray8::new(0x80).convert(),
        );
        assert_eq!(
            SRgb32::new(0.75001144, 0.75001144, 0.75001144),
            SGray16::new(0xC000).convert(),
        );
        assert_eq!(SRgb32::new(0.33, 0.33, 0.33), SGray32::new(0.33).convert(),);
    }

    #[test]
    fn linear_to_srgb() {
        assert_eq!(
            SRgb8::new(0xEF, 0x8C, 0xC7),
            Rgb8::new(0xDC, 0x43, 0x91).convert()
        );
        assert_eq!(
            SRgb8::new(0x66, 0xF4, 0xB5),
            Rgb16::new(0x2205, 0xE699, 0x7654).convert()
        );
        assert_eq!(
            SRgb8::new(0xBC, 0x89, 0xE0),
            Rgb32::new(0.5, 0.25, 0.75).convert()
        );
    }

    #[test]
    fn srgb_to_linear() {
        assert_eq!(
            Rgb8::new(0xDC, 0x43, 0x92),
            SRgb8::new(0xEF, 0x8C, 0xC7).convert(),
        );
        assert_eq!(
            Rgb8::new(0x22, 0xE7, 0x76),
            SRgb16::new(0x6673, 0xF453, 0xB593).convert(),
        );
        assert_eq!(
            Rgb8::new(0x37, 0x0D, 0x85),
            SRgb32::new(0.5, 0.25, 0.75).convert(),
        );
    }

    #[test]
    fn straight_to_premultiplied() {
        assert_eq!(
            Rgba8p::new(0x10, 0x20, 0x40, 0x80),
            Rgba8::new(0x20, 0x40, 0x80, 0x80).convert(),
        );
        assert_eq!(
            Rgba8p::new(0x04, 0x10, 0x20, 0x40),
            Rgba16::new(0x1000, 0x4000, 0x8000, 0x4000).convert(),
        );
        assert_eq!(
            Rgba8p::new(0x60, 0xBF, 0x8F, 0xBF),
            Rgba32::new(0.5, 1.0, 0.75, 0.75).convert(),
        );
    }

    #[test]
    fn premultiplied_to_straight() {
        assert_eq!(
            Rgba8::new(0x40, 0x80, 0xFF, 0x80),
            Rgba8p::new(0x20, 0x40, 0x80, 0x80).convert(),
        );
        assert_eq!(
            Rgba8::new(0x40, 0xFF, 0x80, 0x40),
            Rgba16p::new(0x1000, 0x4000, 0x2000, 0x4000).convert(),
        );
        assert_eq!(
            Rgba8::new(0xAB, 0x55, 0xFF, 0xBF),
            Rgba32p::new(0.5, 0.25, 0.75, 0.75).convert(),
        );
    }

    #[test]
    fn straight_to_premultiplied_srgb() {
        assert_eq!(
            SRgba8p::new(0x16, 0x2A, 0x5C, 0x80),
            SRgba8::new(0x20, 0x40, 0x80, 0x80).convert(),
        );
        assert_eq!(
            SRgba8p::new(0x0D, 0x1C, 0x40, 0x40),
            SRgba16::new(0x2000, 0x4000, 0x8000, 0x4000).convert(),
        );
        assert_eq!(
            SRgba8p::new(0x70, 0xE0, 0xA7, 0xBF),
            SRgba32::new(0.5, 1.0, 0.75, 0.75).convert(),
        );
    }
}
