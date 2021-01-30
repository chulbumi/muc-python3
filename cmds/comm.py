# -*- coding: utf-8 -*-

def delGroup(self, ob, cmd, AllMode = False):
    if ob.Group == None or ob.Group != ob:
        ob.sendLine('☞ 당신을 따르는 무리가 없어요. ^^')        
        return
    from objs.player import is_player
    if cmd == '모두':
        AllMode = True
    cnt = 0 
    for member in ob.follower:
        if AllMode!=True and member['이름'] != cmd:
            continue
        if member in ob.GroupMember:
            member.follow = None
            member.Group = None
            ob.follower.remove(member)
            ob.GroupMember.remove(member)
            ob.sendToGroup('당신의 무리에서 [1m%s[0m[40m[37m%s 제외시킵니다.' % (member['이름'], han_obj(member['이름'])))
            member.sendLine('\r\n[1m%s[0m[40m[37m의 무리에서 당신을 제외시킵니다.' % ob['이름'])
        else:
            member.follow = None
            ob.follower.remove(member)
            ob.sendToGroup('당신이 [1m%s[0m[40m[37m%s 더이상 따라다니지 못하게 합니다.' % (member['이름'], han_obj(member['이름'])))
            member.sendLine('[1m%s[0m[40m[37m의 무리에서 당신을 더이상 따라다니지 못하게 합니다.' % ob['이름'])
        cnt += 1
    if cnt == 0:
        ob.sendLine('☞ 당신을 따르는 그런 대상이 없어요. ^^')
        return
    if ob.Group == ob and len(ob.GroupMember) == 0:
        ob.Group = None       
        ob.GroupMember=[]
