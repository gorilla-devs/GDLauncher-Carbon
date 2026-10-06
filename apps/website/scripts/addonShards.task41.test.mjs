import { test } from "node:test"
import assert from "node:assert/strict"
import * as fs from "node:fs"
import os from "node:os"
import path from "node:path"
import vm from "node:vm"
import { loadShard, prevalidateShards, writeShard, publishShards } from "./addonShards.mjs"
import { mergeAddons } from "./update-addons.mjs"

const fresh = { slug: "fresh", name: "Fresh", imageUrl: null, websiteUrl: "https://example.invalid/fresh" }
function fixture(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "task41-shards-"))
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }))
  const file = path.join(dir, "curseforge-mods.json")
  return { dir, file }
}
for (const bytes of ["{", "[]", "null", '"text"', "42"]) test(`refuses corrupt/invalid existing root ${bytes}`, t => {
  const {dir, file} = fixture(t); fs.writeFileSync(file, bytes)
  assert.throws(() => { const existing = loadShard(dir, "curseforge", "mods"); publishShards(dir, [["curseforge", "mods", mergeAddons(existing, [fresh]).merged]]) }, /Invalid addon shard.*curseforge-mods.json/)
  assert.equal(fs.readFileSync(file, "utf8"), bytes)
})
test("directory read fails with shard context", t => {
  const {dir, file} = fixture(t); fs.mkdirSync(file)
  assert.throws(() => loadShard(dir, "curseforge", "mods"), /Cannot read addon shard.*curseforge-mods.json/)
  assert.ok(fs.statSync(file).isDirectory())
})
test("read ENOENT alone seeds missing history, other read errors remain failures", t => {
  const {dir} = fixture(t)
  assert.deepEqual(loadShard(dir, "curseforge", "mods"), [])
  assert.throws(() => loadShard(dir, "curseforge", "mods", {readFileSync() {throw Object.assign(new Error("inert unreadable"), {code:"EACCES"})}}), /Cannot read addon shard/)
  publishShards(dir, [["curseforge", "mods", [fresh]]])
  assert.equal(JSON.parse(fs.readFileSync(path.join(dir,"curseforge-mods.json"))).fresh.name,"Fresh")
})
test("valid empty and sparse history plus unknown fields survive append/update", t => {
  const {dir, file} = fixture(t); fs.writeFileSync(file, "{}")
  assert.deepEqual(loadShard(dir, "curseforge", "mods"), [])
  fs.writeFileSync(file, '{"history":{"slug":"history","custom":{"kept":true}},"fresh":{"slug":"fresh","extra":"keep"}}')
  const merged = mergeAddons(loadShard(dir, "curseforge", "mods"), [fresh]).merged
  publishShards(dir, [["curseforge", "mods", merged]])
  const saved = JSON.parse(fs.readFileSync(file))
  assert.deepEqual(Object.keys(saved), ["history","fresh"])
  assert.deepEqual(saved.history.custom, {kept:true}); assert.equal(saved.fresh.extra,"keep")
})
test("legal unusual own slugs serialize without prototype mutation", t => {
  const {dir, file} = fixture(t)
  writeShard(dir,"curseforge","mods",[{slug:"__proto__",name:"Proto"},{slug:"constructor",name:"Constructor"},{slug:"toString",name:"String"}])
  const saved = JSON.parse(fs.readFileSync(file)); assert.deepEqual(Object.keys(saved),["__proto__","constructor","toString"])
  assert.equal(saved.__proto__.name,"Proto"); assert.equal(Object.getPrototypeOf(saved),Object.prototype)
})
test("serialization failure leaves old bytes and creates no stage", t => {
  const {dir, file} = fixture(t); fs.writeFileSync(file,"{}\n")
  const circular = {slug:"bad"}; circular.self = circular
  assert.throws(()=>writeShard(dir,"curseforge","mods",[circular]),/circular/)
  assert.equal(fs.readFileSync(file,"utf8"),"{}\n"); assert.deepEqual(fs.readdirSync(dir),["curseforge-mods.json"])
})
for (const fault of ["open","write","rename"]) test(`${fault} failure preserves destination and cleans only its stage`, t => {
  const {dir, file} = fixture(t); const old='{"history":{"slug":"history"}}\n';fs.writeFileSync(file,old)
  const foreign=path.join(dir,".foreign.tmp");fs.writeFileSync(foreign,"foreign")
  const io={...fs}
  if(fault==="open") io.openSync=()=>{throw new Error("inert open failure")}
  if(fault==="write") io.writeFileSync=(fd)=>{fs.writeFileSync(fd,"partial");throw new Error("inert stage failure")}
  if(fault==="rename") io.renameSync=()=>{throw new Error("inert rename failure")}
  assert.throws(()=>writeShard(dir,"curseforge","mods",[fresh],io),/Cannot publish addon shard/)
  assert.equal(fs.readFileSync(file,"utf8"),old);assert.equal(fs.readFileSync(foreign,"utf8"),"foreign")
  assert.deepEqual(fs.readdirSync(dir).sort(),[".foreign.tmp","curseforge-mods.json"])
})
test("exclusive stage collision never removes an unowned temporary file", t => {
  const {dir, file} = fixture(t);fs.writeFileSync(file,"{}")
  let attempted
  const io={...fs,openSync(p,flags){assert.equal(flags,"wx");attempted=p;fs.writeFileSync(p,"unowned");return fs.openSync(p,flags)}}
  assert.throws(()=>writeShard(dir,"curseforge","mods",[fresh],io),/Cannot publish addon shard/)
  assert.equal(fs.readFileSync(file,"utf8"),"{}");assert.equal(fs.readFileSync(attempted,"utf8"),"unowned")
})
test("readers see complete old then complete new content across sibling rename", t => {
  const {dir, file} = fixture(t); const old='{"history":{"slug":"history"}}\n'; fs.writeFileSync(file,old)
  let observed=false
  writeShard(dir,"curseforge","mods",mergeAddons(loadShard(dir,"curseforge","mods"),[fresh]).merged,{...fs,renameSync(stage,dest){
    assert.equal(path.dirname(stage),path.dirname(dest));assert.equal(fs.readFileSync(dest,"utf8"),old)
    assert.deepEqual(Object.keys(JSON.parse(fs.readFileSync(stage))),["history","fresh"]);observed=true;fs.renameSync(stage,dest)
  }})
  assert.ok(observed);assert.deepEqual(Object.keys(JSON.parse(fs.readFileSync(file))),["history","fresh"])
})
test("later corrupt shard is refused before any earlier publication, including dry-run", t => {
  const {dir,file}=fixture(t);fs.writeFileSync(file,"{}\n");const later=path.join(dir,"modrinth-datapacks.json");fs.writeFileSync(later,"{")
  const shards=[["curseforge","mods",[fresh]],["modrinth","datapacks",[fresh]]]
  for(const dryRun of [false,true]) assert.throws(()=>publishShards(dir,shards,{dryRun}),/Invalid addon shard.*modrinth-datapacks.json/)
  assert.equal(fs.readFileSync(file,"utf8"),"{}\n");assert.equal(fs.readFileSync(later,"utf8"),"{")
})
test("dry-run preserves byte-identical history and missing shards", t => {
  const {dir,file}=fixture(t);const bytes='{ "history": { "slug": "history" } }\n';fs.writeFileSync(file,bytes)
  assert.deepEqual(publishShards(dir,[["curseforge","mods",[fresh]],["modrinth","mods",[fresh]]],{dryRun:true}),[])
  assert.equal(fs.readFileSync(file,"utf8"),bytes);assert.deepEqual(fs.readdirSync(dir),["curseforge-mods.json"])
})
// Execute the current CLI main body, changing only input directory and external
// providers to inert handlers. Importing the guarded updater itself has no IO.
function cli(dir,dryRun) {
  let source=fs.readFileSync(new URL("./update-addons.mjs",import.meta.url),"utf8")
  source=source.replace(/^import\s+[\s\S]*?from "[^"]+"\n/gm,"").replace(/^export /gm,"")
  source=source.slice(0,source.lastIndexOf("if (process.argv[1]"))
  source=source.replace(/const __dirname =[^\n]*\n/,"").replace(/const DATA_DIR =[^\n]*\n/,"const DATA_DIR = fixtureDir\n")
  const context=vm.createContext({fixtureDir:dir,loadShard,prevalidateShards,publishShards,normalizeExternalWikiUrl:v=>v,console:{log(){},warn(){},error(){}},process:{argv:dryRun?["node","fixture","--dry-run"]:[],env:{},stdout:{write(){}}},setTimeout})
  vm.runInContext(source+'\nthis.main=main;fetchCurseForge=async()=>{this.fetches++;return [{slug:"fresh",name:"Fresh"}]};fetchModrinth=async()=>{this.fetches++;return []};refreshStaleCurseForge=async()=>0;refreshStaleModrinth=async()=>0;this.fetches=0;',context)
  return context
}
test("actual CLI prevalidates all eleven before inert providers", async t => {
  const {dir,file}=fixture(t);fs.writeFileSync(file,"{}");fs.writeFileSync(path.join(dir,"modrinth-datapacks.json"),"{")
  const c=cli(dir,false);await assert.rejects(c.main(),/Invalid addon shard/);assert.equal(c.fetches,0);assert.equal(fs.readFileSync(file,"utf8"),"{}")
})
test("actual CLI dry-run and serial publisher share the tested store", async t => {
  const {dir,file}=fixture(t);fs.writeFileSync(file,'{"history":{"slug":"history","unknown":true}}\n')
  const before=fs.readFileSync(file);await cli(dir,true).main();assert.deepEqual(fs.readFileSync(file),before);assert.equal(fs.readdirSync(dir).length,1)
  await cli(dir,false).main();assert.equal(fs.readdirSync(dir).length,11);assert.deepEqual(Object.keys(JSON.parse(fs.readFileSync(file))),["history","fresh"])
})
test("close acknowledgement failure still retires the owned stage", t => {
  const {dir,file}=fixture(t);fs.writeFileSync(file,"{}\n")
  let failure
  try { writeShard(dir,"curseforge","mods",[fresh],{...fs,closeSync(fd){fs.closeSync(fd);throw new Error("inert close acknowledgement failure")}}) } catch(e) {failure=e}
  assert.ok(failure);assert.equal(fs.readFileSync(file,"utf8"),"{}\n");assert.deepEqual(fs.readdirSync(dir),["curseforge-mods.json"])
})
