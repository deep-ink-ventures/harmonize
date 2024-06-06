#  import os
#  from uuid import uuid4
#  from coincurve import PublicKey
#  from config import create_safe, get_default_identities, get_wallet_id, assert_ok, assert_err, get_default_principals
#  from web3 import Web3
#
#
#  def test_wallet_lifecycle():
#      wallet_id = get_wallet_id()
#
#      safe = create_safe()
#      principals = get_default_principals()
#
#      # create wallet
#      assert_ok(safe.create_wallet(wallet_id, principals, 1))
#
#      # get wallet
#      wallet = safe.get_wallet(wallet_id)[0][0]
#
#      for p in wallet['signers']:
#          assert p.to_str() in principals
#      assert len(wallet['message_queue']) == 0
#      assert wallet['threshold'] == 1
#
#
#  def test_wallet_not_created_twice():
#      wallet_id = f'test_{str(uuid4())[0:8]}'
#
#      safe = create_safe()
#      principals = [_.sender().to_str() for _ in get_default_identities()]
#
#      # create wallet
#      assert_ok(safe.create_wallet(wallet_id, principals, 1))
#      assert_err(safe.create_wallet(wallet_id, principals, 1), 'WalletAlreadyExists')
#
#
#  def test_signing_lifecycle():
#      wallet_id = get_wallet_id()
#      safe = create_safe()
#      assert_ok(safe.create_wallet(wallet_id, get_default_principals(), 1))
#
#      eth_address = safe.eth_address(wallet_id)[0]['Ok']
#
#      challenge = os.urandom(32)
#      challenge_enc = challenge.hex()
#      safe.propose(wallet_id, challenge_enc)
#      safe.approve(wallet_id, challenge_enc)
#
#      signature_enc = safe.sign(wallet_id, challenge_enc)[0]['Ok']
#      signature = bytes.fromhex(signature_enc)
#
#      # recover in python
#      public_key = PublicKey.from_signature_and_message(signature, challenge, hasher=None)
#      uncompressed_pubkey = public_key.format(compressed=False)
#      keccak_hash = Web3.keccak(uncompressed_pubkey[1:])
#
#      rec_address = keccak_hash[-20:].hex()
#      assert rec_address == eth_address
#
#      # check in canister
#      valid = safe.verify_signature(wallet_id, challenge_enc, signature_enc)
#      assert valid[0]['Ok']
#
#      # check if the message and its metadata have been removed
#      result = safe.get_metadata(wallet_id, challenge_enc)
#      assert_err(result, "MetadataNotFound")
#
#      result = safe.get_messages_with_signers(wallet_id)
#      assert_ok(result)
#      assert len(result[0]['Ok']) == 0
#
#
#  def test_add_remove_signer():
#      wallet_id = get_wallet_id()
#      safe = create_safe()
#      principals = get_default_principals()[:2]
#      new_signer = get_default_principals()[2]
#
#      # Create wallet with initial signers and threshold
#      assert_ok(safe.create_wallet(wallet_id, principals, 1))
#      add_msg = safe.add_signer(wallet_id, new_signer)[0]['Ok']
#
#      # Initially, new signer should not be in the wallet
#      wallet = safe.get_wallet(wallet_id)[0][0]
#      assert new_signer not in [p.to_str() for p in wallet['signers']]
#
#      assert_ok(safe.approve(wallet_id, add_msg))
#      assert_ok(safe.sign(wallet_id, add_msg))
#
#      # After approvals, new signer should be in the wallet
#      wallet = safe.get_wallet(wallet_id)[0][0]
#      assert new_signer in [p.to_str() for p in wallet['signers']]
#      assert len(wallet['signers']) == 3
#
#      # Now, propose to remove a signer
#      signer_to_remove = new_signer
#      remove_msg = safe.remove_signer(wallet_id, signer_to_remove)[0]['Ok']
#
#      assert_ok(safe.approve(wallet_id, remove_msg))
#      assert_ok(safe.sign(wallet_id, remove_msg))
#
#      # After approvals, removed signer should not be in the wallet
#      wallet = safe.get_wallet(wallet_id)[0][0]
#      assert signer_to_remove not in [p.to_str() for p in wallet['signers']]
#
#      # Verify the total number of signers is now reduced by one
#      assert len(wallet['signers']) == 2
#
#
#  def test_change_threshold():
#      wallet_id = get_wallet_id()
#      safe = create_safe()
#      principals = get_default_principals()[:2]
#
#      # Create wallet with initial signers and threshold
#      assert_ok(safe.create_wallet(wallet_id, principals, 1))
#
#      wallet = safe.get_wallet(wallet_id)[0][0]
#      assert wallet['threshold'] == 1
#
#      # Propose to change threshold to 2
#      new_threshold = 2
#      threshold_msg = safe.set_threshold(wallet_id, new_threshold)[0]['Ok']
#
#      assert_ok(safe.approve(wallet_id, threshold_msg))
#      assert_ok(safe.sign(wallet_id, threshold_msg))
#
#      # Verify the new threshold is set
#      wallet = safe.get_wallet(wallet_id)[0][0]
#      assert wallet['threshold'] == new_threshold
#
#
#  def test_principal_wallets_map_update():
#      wallet_id = get_wallet_id()
#      safe = create_safe()
#      principals = get_default_principals()
#      new_signer = get_default_principals()[2]  # Choose a new signer
#
#      # Create wallet with initial signers
#      assert_ok(safe.create_wallet(wallet_id, principals[:2], 1))
#
#      # Check if the wallet is associated with the initial signers
#      for principal in principals[:2]:
#          wallets_for_principal = safe.get_wallets_for_principal(principal)[0]
#          assert wallet_id in wallets_for_principal
#
#      # Add a new signer to the wallet
#      add_msg = safe.add_signer(wallet_id, new_signer)[0]['Ok']
#      assert_ok(safe.approve(wallet_id, add_msg))
#      assert_ok(safe.sign(wallet_id, add_msg))
#
#      # Check if the wallet is now associated with the new signer
#      wallets_for_new_signer = safe.get_wallets_for_principal(new_signer)[0]
#      assert wallet_id in wallets_for_new_signer
#
#      # Remove the new signer from the wallet
#      remove_msg = safe.remove_signer(wallet_id, new_signer)[0]['Ok']
#      assert_ok(safe.approve(wallet_id, remove_msg))
#      assert_ok(safe.sign(wallet_id, remove_msg))
#
#      # Check if the wallet is no longer associated with the removed signer
#      wallets_for_removed_signer = safe.get_wallets_for_principal(new_signer)[0]
#      assert wallet_id not in wallets_for_removed_signer
#
#
#  def test_metadata_lifecycle():
#      wallet_id = get_wallet_id()
#      safe = create_safe()
#      assert_ok(safe.create_wallet(wallet_id, get_default_principals(), 1))
#
#      msg = os.urandom(32).hex()
#      metadata = "test metadata"
#
#      safe.propose(wallet_id, msg)
#
#      # Add metadata to a message
#      assert_ok(safe.add_metadata(wallet_id, msg, metadata))
#
#      # Get metadata for the message
#      result = safe.get_metadata(wallet_id, msg)
#      assert result[0]['Ok'] == metadata
#
#      # Try to add metadata again to the same message
#      result = safe.add_metadata(wallet_id, msg, "new metadata")
#      assert result[0]['Err'] == "Metadata already exists for this message"
#
#
#  def test_propose_with_metadata():
#      wallet_id = get_wallet_id()
#      safe = create_safe()
#      assert_ok(safe.create_wallet(wallet_id, get_default_principals(), 1))
#
#      msg = os.urandom(32).hex()
#      metadata = "test metadata"
#
#      # Propose a message with metadata
#      assert_ok(safe.propose_with_metadata(wallet_id, msg, metadata))
#
#      # Get metadata for the message
#      result = safe.get_metadata(wallet_id, msg)
#      assert result[0]['Ok'] == metadata
