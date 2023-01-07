# Guidelines for adding new stores

# ===============================

Separate each semantic entity into its own store.

Each store should only export the `readonly` store, and some actions to mutate it.
It should NEVER export the store setter directly.
