const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch({executablePath:process.env.EMOTIONAL_BROWSER,headless:true});
  const page = await browser.newPage({viewport:{width:390,height:844},deviceScaleFactor:1});
  await page.goto(process.env.DROP_TEST_URL);
  await page.getByText('Conectado. Selecciona tus archivos.').waitFor();
  await page.setInputFiles('#files',[
    {name:'primero.nes',mimeType:'application/octet-stream',buffer:Buffer.from('synthetic-nes')},
    {name:'segundo.zip',mimeType:'application/octet-stream',buffer:Buffer.from('synthetic-zip')}
  ]);
  if(await page.locator('#send').isEnabled()) throw Error('Ambiguous destination was not required');
  await page.locator('.item').nth(1).locator('select').selectOption('s1');
  if (process.env.DROP_TEST_SCREENSHOT) await page.screenshot({path:process.env.DROP_TEST_SCREENSHOT});
  await page.locator('#send').click();
  await page.getByText('Envío terminado. Revisa el resultado de cada archivo.').waitFor();
  if(await page.locator('.item .ok').count()!==2) throw Error('Uploads did not both succeed');
  await browser.close();
  console.log('Mobile page: two synthetic files sent; ambiguous destination required');
})().catch(e=>{console.error(e);process.exit(1)});
