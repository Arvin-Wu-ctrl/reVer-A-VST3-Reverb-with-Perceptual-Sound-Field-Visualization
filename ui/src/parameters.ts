// SPDX-License-Identifier: MPL-2.0

export const names=['Mix','Pre-delay','Decay','Room Size','Damping','Diffusion','Early / Late','Stereo Width','Wet Pan'];
export const defaults=[.2,.04,.50674647,.5,.5,.7,.4,.8,.5];
export function parameters(root:Element){
 let values=[...(window.__initialParams??defaults)];if(values.length===8)values.push(.5);
 const active=new Set<number>();let begins=0,ends=0;
 const send=(type:string,index:number,value?:number)=>window.ipc?.postMessage(JSON.stringify({type,index,value}));
 const begin=(i:number)=>{if(!active.has(i)){active.add(i);begins++;send('begin',i);}};
 const end=(i:number)=>{if(active.delete(i)){ends++;send('end',i);}};
 names.forEach((name,i)=>{
  const row=document.createElement('div');row.className='control';
  row.innerHTML=`<label for="p${i}">${name}</label><div class="dial" aria-hidden="true"><svg viewBox="0 0 48 48"><path class="track" d="M11.27 36.73A18 18 0 1 1 36.73 36.73"/><path class="progress" d="M11.27 36.73A18 18 0 1 1 36.73 36.73"/></svg><i></i></div><output id="v${i}"></output><input id="p${i}" aria-label="${name}" type="range" min="0" max="1" step="0.001">`;root.append(row);
  const input=row.querySelector('input')!;
  let drag:{x:number;y:number;value:number}|undefined;
  input.addEventListener('pointerdown',e=>{e.preventDefault();input.focus();drag={x:e.clientX,y:e.clientY,value:values[i]};input.setPointerCapture(e.pointerId);begin(i);});
  input.addEventListener('pointermove',e=>{if(!drag||!active.has(i)){drag=undefined;return;}values[i]=Math.round(Math.max(0,Math.min(1,drag.value+((drag.y-e.clientY)+(e.clientX-drag.x))*(e.shiftKey?.0007:.004)))*1000)/1000;input.value=String(values[i]);sync();send('set',i,values[i]);});
  for(const event of ['pointerup','pointercancel','lostpointercapture','blur'])input.addEventListener(event,()=>{drag=undefined;end(i);});
  input.addEventListener('keydown',e=>{if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','PageUp','PageDown'].includes(e.key))begin(i);});
  input.addEventListener('keyup',()=>end(i));
  input.addEventListener('input',()=>{values[i]=+input.value;sync();send('set',i,values[i]);});
 });
 window.addEventListener('blur',()=>[...active].forEach(end));
 function sync(){values.forEach((v,i)=>{const input=root.querySelector<HTMLInputElement>(`#p${i}`)!;if(!active.has(i))input.value=String(v);const row=input.parentElement!;row.style.setProperty('--angle',`${-135+v*270}deg`);row.style.setProperty('--offset',String(84.82*(1-v)));root.querySelector(`#v${i}`)!.textContent=i===1?`${Math.round(v*500)} ms`:i===2?`${(.1*300**v).toFixed(2)} s`:i===8?(Math.abs(v-.5)<.005?'Center':`${Math.round(Math.abs(v-.5)*200)} ${v<.5?'L':'R'}`):`${Math.round(v*100)} %`;});}
 window.setReverbParameters=p=>{if((p.length===8||p.length===9)&&p.every(v=>Number.isFinite(v)&&v>=0&&v<=1)){const next=p.length===8?[...p,.5]:p;next.forEach((v,i)=>{if(!active.has(i))values[i]=v;});sync();}};
 sync();return {get values(){return values;},debug:()=>({targets:values,gestures:{begins,ends,active:[...active]}})};
}
