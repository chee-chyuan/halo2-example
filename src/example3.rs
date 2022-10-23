use std::marker::PhantomData;

// working equality circuit
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{floor_planner::V1, AssignedCell, Layouter, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};

#[derive(Clone)]
struct AddConfig {
    advices: [Column<Advice>; 2],
    selector: Selector,
    instance: Column<Instance>, // instances: [Column<Instance>; 2],
}

struct AddChip<F> {
    config: AddConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> AddChip<F> {
    fn construct(config: AddConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> AddConfig {
        let input = meta.advice_column();
        let output = meta.advice_column();

        let instance = meta.instance_column();

        let selector = meta.selector();

        meta.enable_equality(input);
        meta.enable_equality(output);
        meta.enable_equality(instance);

        meta.create_gate("equal gate", |region| {
            let s = region.query_selector(selector);
            let input_cell = region.query_advice(input, Rotation::cur());
            let output_cell = region.query_advice(output, Rotation::cur());

            vec![s * (input_cell - output_cell)]
        });

        AddConfig {
            advices: [input, output],
            selector,
            instance, // instances: [input_instance, output_instance],
        }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        input_value: Value<F>,
        output_value: Value<F>,
        offset: usize,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        layouter.assign_region(
            || "region",
            |mut region| {
                self.config.selector.enable(&mut region, offset)?;

                let input_cell = region.assign_advice(
                    || "input advise",
                    self.config.advices[0],
                    offset,
                    || input_value,
                )?;
                let output_cell = region.assign_advice(
                    || "output advise",
                    self.config.advices[1],
                    offset,
                    || output_value,
                )?;
                // region.assign_advice_from_instance(annotation, instance, row, advice, offset)

                Ok((input_cell, output_cell))
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        input_cell: AssignedCell<F, F>,
        output_cell: AssignedCell<F, F>,
    ) -> Result<(), Error> {
        let input_res = layouter
            .constrain_instance(input_cell.cell(), self.config.instance, 0)
            .or_else(|e| Err(e));
        if input_res.is_err() {
            return input_res;
        }

        let output_res = layouter
            .constrain_instance(output_cell.cell(), self.config.instance, 1)
            .or_else(|e| Err(e));
        if output_res.is_err() {
            return output_res;
        }
        Ok(())
    }
}

#[derive(Default, Clone)]
struct AddCircuit<F> {
    input: Value<F>,
    output: Value<F>,
}

impl<F: FieldExt> Circuit<F> for AddCircuit<F> {
    type Config = AddConfig;

    type FloorPlanner = V1;

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
    ) -> Result<(), Error> {
        let cs = AddChip::construct(config);
        let (input_cell, output_cell) = cs.assign(
            layouter.namespace(|| "assignment"),
            self.input,
            self.output,
            0,
        )?;
        cs.expose_public(layouter.namespace(|| "instance"), input_cell, output_cell)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use halo2_proofs::{
        circuit::Value,
        dev::MockProver,
        pasta::{EqAffine, Fp},
        plonk::{create_proof, keygen_pk, keygen_vk, verify_proof, SingleVerifier},
        poly::commitment::Params,
        transcript::{Blake2bRead, Blake2bWrite, Challenge255},
    };
    use rand_core::OsRng;

    use super::AddCircuit;
    #[test]
    fn test1() {
        let k = 4;
        let input = Fp::from(1);
        let output = Fp::from(1);

        let circuit = AddCircuit {
            input: Value::known(input),
            output: Value::known(output),
        };

        let public_input = vec![input, output];

        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_example3() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("example3-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Example3 Layout", ("sans-serif", 60)).unwrap();

        let circuit = AddCircuit::<Fp> {
            input: Value::unknown(),
            output: Value::unknown(),
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }

    #[test]
    fn test_real_prover() {
        let k = 4;
        let params: Params<EqAffine> = Params::new(k);

        let empty_circuit: AddCircuit<Fp> = AddCircuit {
            input: Value::unknown(),
            output: Value::unknown(),
        };
        let input = Fp::from(1);
        let output = Fp::from(1);

        let public_input = vec![input, output];

        let circuit = AddCircuit {
            input: Value::known(input),
            output: Value::known(output),
        };

        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        // Create a proof
        create_proof(
            &params,
            &pk,
            &[circuit.clone(), circuit.clone()],
            &[&[&public_input[..]], &[&public_input[..]]],
            OsRng,
            &mut transcript,
        )
        .expect("proof generation should not fail");
        let proof: Vec<u8> = transcript.finalize();

        let strategy = SingleVerifier::new(&params);
        let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        assert!(verify_proof(
            &params,
            pk.get_vk(),
            strategy,
            &[&[&public_input[..]], &[&public_input[..]]],
            &mut transcript,
        )
        .is_ok());
    }
}
