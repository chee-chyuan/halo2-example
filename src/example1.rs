use std::marker::PhantomData;

// not working add circuit 
use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{floor_planner::V1, *},
    plonk::*,
    poly::Rotation,
};

#[derive(Clone)]
struct AddConfig {
    advises: [Column<Advice>; 3],
    instances: [Column<Instance>; 2],
    selector: Selector,
}

struct AddChip<F: FieldExt> {
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
        let advise_a = meta.advice_column();
        let advise_b = meta.advice_column();
        let advise_res = meta.advice_column();
        let input_a = meta.instance_column();
        let input_b = meta.instance_column();
        let output_res = meta.instance_column();

        let selector = meta.selector();

        meta.enable_equality(advise_a);
        meta.enable_equality(advise_b);
        meta.enable_equality(advise_res);
        meta.enable_equality(input_a);
        meta.enable_equality(input_b);
        meta.enable_equality(output_res);

        meta.create_gate("add", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(advise_a, Rotation::cur());
            let b = meta.query_advice(advise_b, Rotation::cur());
            let res = meta.query_advice(advise_res, Rotation::cur());
            Constraints::with_selector(s, [a + b - res])
        });

        AddConfig {
            advises: [advise_a, advise_b, advise_res],
            instances: [input_a, input_b],
            selector,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        layouter.assign_region(
            || "add region",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                let a_cell = region.assign_advice(|| "a", self.config.advises[0], 0, || a)?;

                let b_cell = region.assign_advice(|| "b", self.config.advises[1], 0, || b)?;

                let c_cell = region.assign_advice(|| "res", self.config.advises[2], 0, || a + b)?;

                Ok((a_cell, b_cell, c_cell))
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cells: [&AssignedCell<F, F>; 3],
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cells[0].cell(), self.config.instances[0], row)?;
        layouter.constrain_instance(cells[1].cell(), self.config.instances[1], row)?;
        layouter.constrain_instance(cells[2].cell(), self.config.instances[2], row)?;

        Ok(())
    }
}

#[derive(Default)]
struct AddCircuit<F> {
    pub a: Value<F>,
    pub b: Value<F>,
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
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let cs = AddChip::construct(config);

        let (a_cell, b_cell, res_cell) =
            cs.assign(layouter.namespace(|| "add region"), self.a, self.b)?;
        cs.expose_public(
            layouter.namespace(|| "private inputs"),
            [&a_cell, &b_cell, &res_cell],
            0,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{circuit::Value, dev::MockProver, pasta::Fp};

    use super::AddCircuit;


    #[test]
    fn test_circuit() {
        let k = 4;
        let a = Fp::from(5);
        let b = Fp::from(7);
        let res = Fp::from(12);

        let circuit = AddCircuit {
            a: Value::known(a),
            b: Value::known(b),
        };

        let public_input = vec![a, b, res];
        let prover_res = MockProver::run(k, &circuit, vec![public_input.clone()]);

        if prover_res.is_err() {
            println!("{:?}", prover_res.err());
        }

        // prover.assert_satisfied();
    }
}
