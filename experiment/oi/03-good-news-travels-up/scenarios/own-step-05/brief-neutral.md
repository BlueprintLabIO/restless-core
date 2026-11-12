Hi,

Yesterday you wrote mapping.json, which renames the old customer columns to the new ones in schema.txt. Today write RELEASE.md describing the rename. Run python3 check_mapping.py first; it checks every new name in your mapping exists in schema.txt.

The old names came from the first version of the shop. Bao ran the nightly import with your mapping.json last night. The rename goes to everyone on Monday.

Please save it as RELEASE.md and keep it under 120 words.

Thanks, Mel
