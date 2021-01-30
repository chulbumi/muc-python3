# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

#무리 추가
    def delParty(self, ob, cmd, AllMode = False):
        if ob.Party == None or ob.Party != ob:
            ob.sendLine('☞ 당신을 따르는 무리가 없어요. ^^')        
            return

        dummy = copy.copy(ob.follower)
        if cmd == '모두':
            AllMode = True
        cnt = 0 
        for member in dummy:
            if AllMode!=True and member['이름'] != cmd:
                continue
            if member in ob.PartyMember:
                member.follow = None
                member.Party = None
                ob.follower.remove(member)
                ob.PartyMember.remove(member)
                ob.sendToParty('당신의 무리에서 [1m%s[0m[40m[37m%s 제외시킵니다.' % (member['이름'], han_obj(member['이름'])), prompt = True)
                member.sendLine('\r\n[1m%s[0m[40m[37m의 무리에서 당신을 제외시킵니다.' % ob['이름'])
                member.lpPrompt()
            else:
                member.follow = None
                ob.follower.remove(member)
                ob.sendToParty('당신이 [1m%s[0m[40m[37m%s 더이상 따라다니지 못하게 합니다.' % (member['이름'], han_obj(member['이름'])), prompt = True)
                member.sendLine('\r\n[1m%s[0m[40m[37m의 무리에서 당신을 더이상 따라다니지 못하게 합니다.' % ob['이름'])
                member.lpPrompt()
            cnt += 1
        if cnt == 0:
            ob.sendLine('☞ 당신을 따르는 그런 대상이 없어요. ^^')
            return
        if ob.Party == ob and len(ob.PartyMember) == 0:
            ob.Party = None       
            ob.PartyMember=[]
            
    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [동행원|무리원] 제외')
            return
        if ob.Party == None:
            ob.sendLine('☞ 당신이 속한 무리가 없어요. ^^')
        else:
            self.delParty(ob, line)

