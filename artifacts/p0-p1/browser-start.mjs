const task = await taskSpace('YDLite P0 P1 and segmented tabs');
console.log({spaceId:task.spaceId});
console.log(await task.page('p1').snapshot());
