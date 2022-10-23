// extend upon example4 to perform 2 additions using AddChip

use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{floor_planner::V1, Layouter, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Instance, Selector},
    poly::Rotation,
};

use crate::example4_gadget::{AddChip, AddConfig};

// add a+b+c

#[derive(Clone)]
struct Add2Config {
    advices: [Column<Advice>; 4],
    selector: Selector,
    instance: Column<Instance>,
    add_config: AddConfig,
}

struct Add2Chip<F> {
    config: Add2Config,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> Add2Chip<F> {
    fn construct(config: Add2Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Add2Config {
        let a_advice = meta.advice_column();
        let b_advice = meta.advice_column();
        let c_advice = meta.advice_column();
        let res_advice = meta.advice_column();
        let instance = meta.instance_column();
        let selector = meta.selector();

        let add_config = AddChip::configure(meta, a_advice, b_advice, res_advice, instance);

        // a       b   res_1
        // res_1   c    res
        meta.create_gate("add 2 constraint", |region| {
            let a = region.query_advice(a_advice, Rotation::cur());
            let b = region.query_advice(b_advice, Rotation::cur());
            let c = region.query_advice(c_advice, Rotation::next());
            let res = region.query_advice(res_advice, Rotation::next());

            let s = region.query_selector(selector);

            vec![s * (a + b + c - res)]
        });

        Add2Config {
            advices: [a_advice, b_advice, c_advice, res_advice],
            selector,
            instance,
            add_config,
        }
    }

    fn assign(&self, mut layouter: impl Layouter<F>, a: Value<F>, b: Value<F>, c: Value<F>) {
        let add_cs_1 = AddChip::<F>::construct(self.config.add_config.clone());
        let (a_cell, b_cell, c_cell) = add_cs_1
            .assign(layouter.namespace(|| "intermediate assign"), a, b, 0)
            .unwrap();
        add_cs_1
            .expose_public(layouter.namespace(|| "pub"), a_cell, b_cell, c_cell.clone())
            .unwrap();

        let add_cs_2 = AddChip::<F>::construct(self.config.add_config.clone());
        let res_1_value = c_cell.value().copied();

        let (res1_cell, c_cell, res_cell) = add_cs_2
            .assign(layouter.namespace(|| "result assign"), res_1_value, c, 1)
            .unwrap();

        layouter
            .constrain_instance(res_cell.cell(), self.config.instance, 3)
            .unwrap();

        layouter
            .assign_region(
                || "contrain res 1",
                |mut region| region.constrain_equal(c_cell.cell(), res1_cell.cell()),
            )
            .unwrap();
    }
}

#[derive(Default)]
struct Add2Circuit<F> {
    a: Value<F>,
    b: Value<F>,
    c: Value<F>,
}

impl<F: FieldExt> Circuit<F> for Add2Circuit<F> {
    type Config = Add2Config;

    type FloorPlanner = V1;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut halo2_proofs::plonk::ConstraintSystem<F>) -> Self::Config {
        Add2Chip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl halo2_proofs::circuit::Layouter<F>,
    ) -> Result<(), halo2_proofs::plonk::Error> {
        let cs = Add2Chip::<F>::construct(config);

        cs.assign(layouter.namespace(|| "add 2"), self.a, self.b, self.c);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{pasta::Fp, circuit::Value, dev::MockProver};

    use super::Add2Circuit;

    #[test]
    fn test() {
        let k = 4;
        let a = Fp::from(5);
        let b = Fp::from(7);
        let c = Fp::from(12);
        let res = a + b + c;

        let circuit = Add2Circuit {
            a: Value::known(a),
            b: Value::known(b),
            c: Value::known(c),
        };

        let pub_instances = vec![a, b, c, res];

        let prover = MockProver::run(k, &circuit, vec![pub_instances.clone()]).unwrap();
        prover.assert_satisfied();
    }
}