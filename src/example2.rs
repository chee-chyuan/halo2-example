use std::marker::PhantomData;

// not working simple equality circuit
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Instance, Selector},
    poly::Rotation,
};

#[derive(Clone)]
struct AddConfig {
    pub advice: [Column<Advice>; 2],
    pub selector: Selector,
    pub instance: [Column<Instance>; 2],
}

struct AddChip<F: FieldExt> {
    config: AddConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> AddChip<F> {
    pub fn construct(config: AddConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> AddConfig {
        let input = meta.advice_column();
        let output = meta.advice_column();
        let selector = meta.selector();

        let input_instance = meta.instance_column();
        let output_instance = meta.instance_column();

        meta.enable_equality(input);
        meta.enable_equality(input_instance);
        meta.enable_equality(output);
        meta.enable_equality(output_instance);

        meta.create_gate("equal gate", |meta| {
            let a = meta.query_advice(input, Rotation::cur());
            let b = meta.query_advice(output, Rotation::cur());
            let s = meta.query_selector(selector);
            vec![s * (a - b)]
        });

        AddConfig {
            advice: [input, output],
            selector,
            instance: [input_instance, output_instance],
        }
    }
}

#[derive(Default)]
struct AddCircuit<F> {
    pub input: Value<F>,
    pub output: Value<F>,
}

impl<F: FieldExt> Circuit<F> for AddCircuit<F> {
    type Config = AddConfig;

    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        AddChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        // let cs = AddChip::construct(config);

        let i = layouter.assign_region(
            || "region",
            |mut region| {
                config.selector.enable(&mut region, 0)?;
                let input_cell =
                    region.assign_advice(|| "input", config.advice[0], 0, || self.input)?;
                let output_cell =
                    region.assign_advice(|| "output", config.advice[1], 0, || self.output)?;

                Ok([input_cell, output_cell])
            },
        )?;

        layouter.constrain_instance(i[0].cell(), config.instance[0], 0)?;
        layouter.constrain_instance(i[1].cell(), config.instance[1], 0)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{
        circuit::{self, Value},
        dev::MockProver,
        pasta::Fp,
    };

    use super::AddCircuit;

    #[test]
    fn test1() {
        let k = 4;
        let input = Fp::from(5);
        let output = Fp::from(5);

        let circuit = AddCircuit {
            input: Value::known(input),
            output: Value::known(output),
        };

        let public_input = vec![input, output];

        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();
    }
}
