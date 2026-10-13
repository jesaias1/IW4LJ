# Family identity and composition

`asset_core::FamilyId` identifies IW4, IW5, T5 and T6. `AssetNamespace`,
`ZoneGame` and script `Realm` name this same type at their respective boundaries.
It has no default. Capture builders start without a family; capture requires
explicit selection. Failed map opens return a refusal and publish `MapLoadFailed`.
Empty worlds require an explicit family draw policy.

Assets retain their captured family through native compilation and publication.
T6 weapon rows, common meshes, clips, materials and sounds stay T6. Missing native
components refuse dependent capabilities. Another family's row or material
surface cannot stand in for them. T6 sound spatial policy uses captured native
alias flags and dry/near curves; six-speaker pans are explicitly projected into
the host stereo output.

The map selects soldier kits. A soldier's body and first-person hands belong to
the map's family. Kit arms take precedence over weapon-authored hands; authored
hands are eligible only when they belong to the soldier's family. Missing hands
leave first-person composition unresolved. T6 maps capture both faction viewhands
and require the selected kit hands for every weapon family. `SoldierPresentations`
prepares each selected kit once, retaining its body, native animation profile,
published mesh owner and checked hand mounts. `SoldierFpvPresentation` owns
hand admission; unused WeaponDef hands are optional. Head capabilities distinguish
absence, bound family/pose/mount and refusal; required collision refuses invalid heads.

`FamilyFpvMesh<F>` is issued by a published catalog after checking family and
owner. `SoldierFpvConnection<G, H>` connects native gun family G to native soldier
hands family H for IW4, IW5, T5 and T6. Both meshes must have the same owner.
`NativeFpvConnection<F>` names the same-family case. Attachments and auxiliary
models must belong to the weapon's family. The assembled skeleton validates
mounts; missing hands or mount tags refuse composition without substitute models.

Third-person body clips, animation sources, trees and scripts must match the
soldier's family. Missing foreign profiles refuse; IW4 clips are not a fallback.
The prepared body profile supplies simulation trees and persistent remote
animation; rendering does not rebuild character bindings per entity.
World policies provide family-specific sky, lighting and shadow inputs. Material
adapters compile those inputs and authored state into shared renderer products.

Simulation still uses the existing host rules. This boundary does not introduce
native rules per game. Weapon rows still contain family-specific extension fields;
registry binding and renderer code banks retain their existing interfaces.

The session weapon compiler traverses one published registry to build complete
simulation rows. `SimWeaponContent` validates dense row order and script alias
targets and derives readiness from execution results, retaining refusal reasons.
It publishes immutable execution arrays. Match installation checks
the registry revision and exact product owner. Class projection and the session
manifest reject foreign publications and preserve execution refusals. `SimContentBuilder::bootstrap` keeps the empty
world explicit; installed matches use `for_match` with a compiled product.
