import hashlib
import unittest

from src.settlement import (
    SettlementError,
    SettlementRecord,
    SettlementState,
    compute_payment_hash,
    compute_proof_hash,
)


class SettlementTests(unittest.TestCase):
    def test_deterministic_hashes(self) -> None:
        task_id = "task-123"
        token_id = "token-abc"
        expected_payment = hashlib.sha256(
            (task_id + token_id + "payment").encode("utf-8")
        ).hexdigest()
        payment_hash = compute_payment_hash(task_id, token_id)
        self.assertEqual(payment_hash, expected_payment)

        expected_proof = hashlib.sha256(
            (task_id + payment_hash + "proof").encode("utf-8")
        ).hexdigest()
        proof_hash = compute_proof_hash(task_id, payment_hash)
        self.assertEqual(proof_hash, expected_proof)

    def test_state_transitions(self) -> None:
        payment_hash = compute_payment_hash("task-1", "token-1")
        proof_hash = compute_proof_hash("task-1", payment_hash)
        record = SettlementRecord(
            task_id="task-1",
            token_id="token-1",
            payment_hash=payment_hash,
            proof_hash=proof_hash,
            btcpay_invoice_id="inv-1",
            btcpay_invoice_url="https://example.com/i/inv-1",
            state=SettlementState.Requested,
            requested_at=1,
        )
        record.mark_locked(2)
        self.assertEqual(record.state, SettlementState.LockedAcked)
        record.mark_claimed(proof_hash, 3)
        self.assertEqual(record.state, SettlementState.Claimed)
        self.assertEqual(record.claimed_at, 3)

    def test_proof_before_lock_rejected(self) -> None:
        payment_hash = compute_payment_hash("task-2", "token-2")
        proof_hash = compute_proof_hash("task-2", payment_hash)
        record = SettlementRecord(
            task_id="task-2",
            token_id="token-2",
            payment_hash=payment_hash,
            proof_hash=proof_hash,
            btcpay_invoice_id="inv-2",
            btcpay_invoice_url="https://example.com/i/inv-2",
            state=SettlementState.Requested,
            requested_at=1,
        )
        with self.assertRaises(SettlementError) as ctx:
            record.mark_claimed(proof_hash, 2)
        self.assertEqual(ctx.exception.code, "proof_before_lock")


if __name__ == "__main__":
    unittest.main()
