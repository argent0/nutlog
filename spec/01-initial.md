`nutlog` is a CLI tool to log information about the food the user has bought.

# Technical details

**Programming language**: rust
**Interface**: Command Line Interface
**Output**: Human readble and JSON (`--json` flag)
**Database**: SQLite
**Command Name**: nutlog

Money amount are stored as cents using integers.

# Objective

It is linux-only. CLI forever.

It is single user only. The user will install it using the package manager of
their distro.

It is LLM-agent first. Users will usually interact with the tool using a LLM
that will, in turn, use a skill to figure out how to store the information.

# User stories

## Products

**The user can store a new product**

nutlog product create "YOUGURISIMO 300G NATU" --product-tags "yogurt"

**The user can list products in JSON format**

```bash
nutlog --json product list
```

```
{
    id: //...
    name:
    product_tags: [ {}, ],
    nutritional_information: {
    },
    // JSON schema to be defined
}
```

**The user can search products by name**

It performs fuzzy search and ranks the results.

```bash
nutlog --json product search --name "yogu"
```

**The user can edit products**

```bash
nutlog product tag add <product-id> --tag-id <tag_id>
```

```bash
nutlog product rename <product-id> --name "xxx"
```

**Show product**
```bash
nutlog product show <product-id>
```

## Nutritional information

**Products can be linked to nutritional information**

```
{
    "reference_quantity": 100,
    "reference_units": "g",
    "energy_kcal": 123,
    "protein_g": 8.2,
    "carbohydrates_g": 12.5,
    "fat_g": 3.1,
    "fiber_g": 1.2,
    "sugars_g": 4.8,
    "micronutrients": [
        { "nutrient_id": 42, "amount": 0.8, "unit": "mg" }
    ]
}
```

## Nutrients

Nutrient:

```json
{
    id:
    name:
    recomended_intake:
    // more non-schema defined fields with information about the nutrient
    // found in [organes, lemons]
}
```

There is a Nutrient CRM.

## Product Tags

**The user can search products by tag**

```bash
nutlog --json product search --product-tag "yogurt"
```

**The user can read/create/delete tags** 

An independent entity in the schema

```bash
nutlog product-tag search "yo" # fussy search and rank result
nutlog product-tag create "yogurt"
nutlog product-tag delete <tag-id>
nutlog product-tag show <tag-id>
```

## Purchases

**The user can register product purchases**

Basically the user purchased product X at price Y in the store Z at time T. Price and store are optional.

```bash
nutlog purchase create <product-id> --price "1999.00" --store <store-id> --date 01-02-2026 --quantity 5
```

## Stores

There is a CRM for stores.

```
nutlog store create
nutlog store show <store-id>
nutlog store tag add <store-tag-id>
```

## Store tags

There is a CRM for stores-tags.

## Consumption log

When a product is consumed.

```
nutlog consumption create <product-id>
```

# Consumption

It' will be assumed that everything bought is eaten.

# Database

**Location**: `SQLite` database stored in a configurable location. Default to a proper XDG folder.
**Timestamps** : in UTC. Converted to the local timezone in Human Readable output, And optionally in JSON.
