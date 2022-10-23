use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{floor_planner::V1, AssignedCell, Layouter, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};

// a + b = c
#[derive(Clone)]
pub struct AddConfig {
    advices: [Column<Advice>; 3],
    selector: Selector,
    instance: Column<Instance>,
}

pub struct AddChip<F> {
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

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        a_advice: Column<Advice>,
        b_advice: Column<Advice>,
        res_advice: Column<Advice>,
        instance: Column<Instance>,
    ) -> AddConfig {
        let selector = meta.selector();

        meta.enable_equality(a_advice);
        meta.enable_equality(b_advice);
        meta.enable_equality(res_advice);
        meta.enable_equality(instance);

        meta.create_gate("addition gate", |region| {
            let s = region.query_selector(selector);

            let a = region.query_advice(a_advice, Rotation::cur());
            let b = region.query_advice(b_advice, Rotation::cur());
            let res = region.query_advice(res_advice, Rotation::cur());

            vec![s * (a + b - res)]
        });

        AddConfig {
            advices: [a_advice, b_advice, res_advice],
            selector,
            instance,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        offset: usize,
    ) -> Result<(AssignedCell<F, F>, AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        let res = a + b;

        layouter.assign_region(
            || "add region",
            |mut region| {
                self.config.selector.enable(&mut region, offset)?;

                let a_cell =
                    region.assign_advice(|| "a input", self.config.advices[0], offset, || a)?;
                let b_cell =
                    region.assign_advice(|| "b input", self.config.advices[1], offset, || b)?;
                let res_cell =
                    region.assign_advice(|| "res", self.config.advices[2], offset, || res)?;

                Ok((a_cell, b_cell, res_cell))
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        a_cell: AssignedCell<F, F>,
        b_cell: AssignedCell<F, F>,
        res_cell: AssignedCell<F, F>,
    ) -> Result<(), Error> {
        layouter.constrain_instance(a_cell.cell(), self.config.instance, 0)?;
        layouter.constrain_instance(b_cell.cell(), self.config.instance, 1)?;
        layouter.constrain_instance(res_cell.cell(), self.config.instance, 2)?;

        Ok(())
    }
}

#[derive(Default)]
struct AddCircuit<F> {
    a: Value<F>,
    b: Value<F>,
}

impl<F: FieldExt> Circuit<F> for AddCircuit<F> {
    type Config = AddConfig;

    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let a_advice = meta.advice_column();
        let b_advice = meta.advice_column();
        let res_advice = meta.advice_column();

        let instance = meta.instance_column();
        AddChip::configure(meta, a_advice, b_advice, res_advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        let cs = AddChip::construct(config);

        let (a_cell, b_cell, res_cell) =
            cs.assign(layouter.namespace(|| "assigning"), self.a, self.b, 0)?;
        cs.expose_public(layouter.namespace(|| "instance"), a_cell, b_cell, res_cell)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{circuit::Value, dev::MockProver, pasta::Fp};

    use super::AddCircuit;

    #[test]
    fn test() {
        let k = 4;
        let a = Fp::from(5);
        let b = Fp::from(1);
        let res = a + b;

        let circuit = AddCircuit {
            a: Value::known(a),
            b: Value::known(b),
        };
        let public_input = vec![a, b, res];

        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();
    }
}
