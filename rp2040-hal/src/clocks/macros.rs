macro_rules! clock {
    {
        $(#[$attr:meta])*
        struct $name:ident {
            reg: $reg:ident,
            default_src: $default_src:ident,
            src: {$($src:ident: $src_variant:ident),*},
            use_aux_src: $use_aux_src:ident,
            auxsrc: {$($auxsrc:ident: $aux_variant:ident),*}
        }
     } => {
        base_clock!{
            $(#[$attr])*
            ($name, $reg, auxsrc={$($auxsrc: $aux_variant),*})
        }

        divisable_clock!($name, $reg);

        $crate::paste::paste!{
            $(impl ValidSrc<$name> for $src {
                type Variant = [<$reg:camel SrcType>];

                fn is_aux(&self) -> bool{
                    false
                }
                fn variant(&self) -> [<$reg:camel SrcType>] {
                    [<$reg:camel SrcType>]::Src(pac::clocks::[<$reg _ctrl>]::SRC_A::$src_variant)
                }
            })*

            impl GlitchlessClock for $name {
                type Clock = Self;

                fn await_select(&self, clock_token: &ChangingClockToken<Self>) -> nb::Result<(),()> {
                    let shared_dev = unsafe { self.shared_dev.get() };

                    let selected = shared_dev.[<$reg _selected>].read().bits();
                    if selected != 1 << clock_token.clock_nr {
                        return Err(nb::Error::WouldBlock);
                    }

                    Ok(())
                }
            }

            /// Holds register value for ClockSource for this clock
            pub enum [<$reg:camel SrcType>] {
                /// Its an clock source that is to be used as source
                Src(pac::clocks::[<$reg _ctrl>]::SRC_A),
                /// Its an clock source that is to be used as aux source
                Aux(pac::clocks::[<$reg _ctrl>]::AUXSRC_A)
            }

            impl [<$reg:camel SrcType>] {
                fn get_clock_id(&self) -> u8 {
                    match self {
                        Self::Src(v) => *v as u8,
                        Self::Aux(v) => *v as u8,
                    }
                }

                fn unwrap_src(&self) -> pac::clocks::[<$reg _ctrl>]::SRC_A {
                    match self {
                        Self::Src(v) => *v,
                        Self::Aux(_) => $use_aux_src,
                    }
                }

                fn unwrap_aux(&self) -> pac::clocks::[<$reg _ctrl>]::AUXSRC_A {
                    match self {
                        Self::Src(_) => panic!(),
                        Self::Aux(v) => *v
                    }
                 }
            }

            impl $name {
                /// WIP - Helper function to reset source (blocking)
                pub fn reset_source_await(&mut self) -> nb::Result<(), ()> {
                    let shared_dev = unsafe { self.shared_dev.get() };

                    shared_dev.[<$reg _ctrl>].write(|w| w.src().variant($default_src));

                    self.await_select(&ChangingClockToken{clock_nr:0, clock: PhantomData::<Self>})
                }

                fn set_src<S:ClockSource + ValidSrc<$name, Variant=[<$reg:camel SrcType>]>>(&mut self, src: &S)-> ChangingClockToken<$name> {
                    let shared_dev = unsafe { self.shared_dev.get() };

                    shared_dev.[<$reg _ctrl>].modify(|_,w| {
                        w.src().variant(src.variant().unwrap_src())
                    });

                    ChangingClockToken {
                        clock: PhantomData::<$name>,
                        clock_nr: src.variant().get_clock_id(),
                    }
                }

                fn set_self_aux_src(&mut self) -> ChangingClockToken<$name> {
                    unsafe { self.shared_dev.get() }.[<$reg _ctrl>].modify(|_, w| {
                        w.src().variant($use_aux_src)
                    });

                    ChangingClockToken{
                        clock: PhantomData::<$name>,
                        clock_nr: $use_aux_src as u8,
                    }
                }

                /// Configure this clock based on a clock source and desired frequency
                pub fn configure_clock<S:ClockSource + ValidSrc<$name, Variant=[<$reg:camel SrcType>]>>(&mut self, src: &S, freq: Hertz) -> bool{
                    let src_freq: Hertz  = src.get_freq();

                    if freq .gt(& src_freq){
                        return false;
                    }

                    let div = make_div(src_freq, freq).unwrap();

                    // If increasing divisor, set divisor before source. Otherwise set source
                    // before divisor. This avoids a momentary overspeed when e.g. switching
                    // to a faster source and increasing divisor to compensate.
                    if div > self.get_div() {
                        self.set_div(div);
                    }

                    // Set aux mux first, and then glitchless src mux.
                    let token = if src.is_aux() {
                        // If switching to another aux source, switch away from aux *first* to avoid
                        // passing glitches when changing the aux mux.
                        // This *assumes* that glitchless source 0 is no faster than the aux source.
                        // TODO: Ideally also only do this if the current source is aux
                        nb::block!(self.reset_source_await()).unwrap();

                        self.set_aux(src);
                        self.set_self_aux_src()
                    } else {
                        self.set_src(src)
                    };

                    nb::block!(self.await_select(&token)).unwrap();


                    // Now that the source is configured, we can trust that the user-supplied
                    // divisor is a safe value.
                    self.set_div(div);

                    // Store the configured frequency
                    self.frequency = make_frequency(src_freq, div).unwrap();

                    true
                }
            }
        }
    };
    {
        $( #[$attr:meta])*
        struct $name:ident {
            reg: $reg:ident,
            auxsrc: {$($auxsrc:ident: $variant:ident),*},
            div: false
        }
    } => {
        base_clock!{
            $(#[$attr])*
            ($name, $reg, auxsrc={$($auxsrc: $variant),*})
        }

        // Just to match proper divisable clocks so we don't have to do something special in configure function
        impl ClockDivision for $name {
            fn set_div(&mut self, _: u32) {}
            fn get_div(&self) -> u32 {1}
        }

        stoppable_clock!($name, $reg);
    };
    {
        $( #[$attr:meta])*
        struct $name:ident {
            reg: $reg:ident,
            auxsrc: {$($auxsrc:ident: $variant:ident),*}
        }
    } => {
        base_clock!{
            $(#[$attr])*
            ($name, $reg, auxsrc={$($auxsrc: $variant),*})
        }

        divisable_clock!($name, $reg);
        stoppable_clock!($name, $reg);
    };
}

macro_rules! divisable_clock {
    ($name:ident, $reg:ident) => {
        $crate::paste::paste! {
            impl ClockDivision for $name {
                fn set_div(&mut self, div: u32) {
                    unsafe { self.shared_dev.get() }.[<$reg _div>].modify(|_, w| unsafe {
                        w.bits(div);
                        w
                    });
                }
                fn get_div(&self) -> u32 {
                    unsafe { self.shared_dev.get() }.[<$reg _div>].read().bits()
                }
                // TODO: Implement get_div_integer() and get_div_fractional()
            }
        }
    };
}

macro_rules! stoppable_clock {
    ($name:ident, $reg:ident) => {
        $crate::paste::paste!{
            /// Holds register value for ClockSource for this clock
            pub enum [<$reg:camel SrcType>] {
                /// Its an clock source that is to be used as aux source
                Aux(pac::clocks::[<$reg _ctrl>]::AUXSRC_A)
            }

            impl [<$reg:camel SrcType>] {
                fn unwrap_aux(&self) -> pac::clocks::[<$reg _ctrl>]::AUXSRC_A {
                   match self {
                       Self::Aux(v) => *v
                   }
                }
            }

            impl StoppableClock for $name {
                fn enable(&mut self) {
                    unsafe { self.shared_dev.get() }.[<$reg _ctrl>].modify(|_, w| {
                        w.enable().set_bit()
                    });
                }

                fn disable(&mut self) {
                    unsafe { self.shared_dev.get() }.[<$reg _ctrl>].modify(|_, w| {
                        w.enable().clear_bit()
                    });
                }

                fn kill(&mut self) {
                    unsafe { self.shared_dev.get() }.[<$reg _ctrl>].modify(|_, w| {
                        w.kill().set_bit()
                    });
                }
            }

            impl $name {
                /// Configure this clock based on a clock source and desired frequency
                pub fn configure_clock<S:ClockSource + ValidSrc<$name, Variant=[<$reg:camel SrcType>]>>(&mut self, src: &S, freq: Hertz) -> bool{
                    let src_freq: Hertz  = src.get_freq();

                    if freq .gt(& src_freq){
                        return false;
                    }

                    let div = make_div(src_freq, freq).unwrap();

                    // If increasing divisor, set divisor before source. Otherwise set source
                    // before divisor. This avoids a momentary overspeed when e.g. switching
                    // to a faster source and increasing divisor to compensate.
                    if div > self.get_div() {
                        self.set_div(div);
                    }

                    // If no glitchless mux, cleanly stop the clock to avoid glitches
                    // propagating when changing aux mux. Note it would be a really bad idea
                    // to do this on one of the glitchless clocks (clk_sys, clk_ref).

                    // Disable clock. On clk_ref and clk_sys this does nothing,
                    // all other clocks have the ENABLE bit in the same position.
                    self.disable();
                    if (self.frequency > 0u32.Hz()) {
                        // Delay for 3 cycles of the target clock, for ENABLE propagation.
                        // Note XOSC_COUNT is not helpful here because XOSC is not
                        // necessarily running, nor is timer... so, 3 cycles per loop:
                        let sys_freq = 125_000_000; // TODO
                        let delay_cyc = (sys_freq / *self.frequency.integer()) + 1;
                        cortex_m::asm::delay(delay_cyc);
                    }

                    // Set aux mux.
                    self.set_aux(src);

                    // Enable clock. On clk_ref and clk_sys this does nothing,
                    // all other clocks have the ENABLE bit in the same position.
                    self.enable();

                    // Now that the source is configured, we can trust that the user-supplied
                    // divisor is a safe value.
                    self.set_div(div);

                    // Store the configured frequency
                    self.frequency = make_frequency(src_freq, div).unwrap();
                    true
                }
            }
        }
    };
}

macro_rules! base_clock {
    {
        $(#[$attr:meta])*
        ($name:ident, $reg:ident, auxsrc={$($auxsrc:ident: $variant:ident),*})
    } => {
        $crate::paste::paste!{

            $(impl ValidSrc<$name> for $auxsrc {
                type Variant = [<$reg:camel SrcType>];

                fn is_aux(&self) -> bool{
                    true
                }
                fn variant(&self) -> [<$reg:camel SrcType>] {
                    [<$reg:camel SrcType>]::Aux(pac::clocks::[<$reg _ctrl>]::AUXSRC_A::$variant)
                }
            })*

            impl ClocksManager {
                    #[ doc = "Getter for the" $name ]
                    pub fn [<$name:snake>](&self) -> $name {

                        //TODO: Init clock here
                        $name {
                            shared_dev: self.shared_clocks,
                            frequency: 0.Hz(),
                        }
                    }

            }
            $(#[$attr])*
            pub struct $name {
                shared_dev: ShareableClocks,
                frequency: Hertz,
            }

            impl $name {
                /// Returns the frequency of the configured clock
                pub fn freq(&self) -> Hertz {
                    self.frequency
                }

                fn set_aux<S:ClockSource + ValidSrc<$name, Variant=[<$reg:camel SrcType>]>>(&mut self, src: &S) {
                    let shared_dev = unsafe { self.shared_dev.get() };

                    shared_dev.[<$reg _ctrl>].modify(|_,w| {
                        w.auxsrc().variant(src.variant().unwrap_aux())
                    });
                }
            }

            impl Sealed for $name {}

            impl From<$name> for Hertz
             {
                fn from(value: $name) -> Hertz {
                    value.frequency
                }
            }
        }
    };
}
