const vm=require('node:vm'),crypto=require('node:crypto');
let input='';process.stdin.setEncoding('utf8');process.stdin.on('data',x=>input+=x);
process.stdin.on('end',()=>{const context=vm.createContext({s:{createHash:crypto.createHash}});
const result=new vm.Script(input+'; JSON.stringify(K)').runInContext(context,{timeout:1000});
process.stdout.write(result);});