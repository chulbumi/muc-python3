# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def addParty(self, ob, cmd, AllMode = False):
        if ob.Party != None and ob.Party!=ob:
            ob.sendLine ('☞ 이미 당신은 무리중이여서 무리를 만들 수 없어요. ^^')
            return
        ob.Party = ob
        #self.PartyMember.append(self)
        from objs.player import is_player
        if cmd == '모두':
            AllMode = True
        cnt1 = 0 
        cnt2 = 0
        cnt3 = 0
        for member in ob.follower:
            if AllMode!=True and member['이름'] != cmd:
                continue
            cnt2 += 1
            if member == None or is_player(member) == False:
                if AllMode == False:
                    ob.sendLine('%s 이곳에 없어서 무리에 참여하지 못합니다.' % member.han_un())
                continue
            if member.follow != ob:
                if AllMode == False:
                    ob.sendLine('%s 이미 따르는자가 있습니다.' % member.han_un())
                continue
            if member.Party != None:
                if AllMode == False:
                    ob.sendLine('%s 따르는 무리가 있으므로 안됩니다.' % member.han_obj())
                    cnt3+=1
                continue
            '''
            if ob['소속'] != member['소속']:
                if AllMode == False:
                    self.sendLine('%s 다른 정파의 무림인이므로 무리활동을 할 수 없습니다.' % member['이름'].han_un())
                    self.lpPrompt()
                    cnt3+=1
                continue
            '''
            member.Party=ob
            ob.PartyMember.append(member)
            #member.PartyMember.append(member)
            ob.sendLine('당신의 무리에 [1m%s[0m[40m[37m%s 들어옵니다.' % (member['이름'], han_iga(member['이름'])))
            member.sendLine('\r\n당신이 [1m%s[0m[40m[37m의 무리에 들어갑니다.' % ob['이름'])
            member.lpPrompt()
            ob.sendRoom('[1m%s[0m[40m[37m%s [1m%s[0m[40m[37m의 무리에 들어갑니다.'% (member['이름'], han_iga(member['이름']), ob['이름']), ex = member)
            cnt1 += 1
        if cnt2 == 0:
            ob.sendLine('☞ 당신을 따르는 대상이 아닙니다.')
        elif cnt1 == 0 and cnt3 == 0:
            ob.sendLine('☞ 추가된 무리원이 없습니다.')

    def getHPbar(self, ob):
        strAnsi=['[0m[41m[30m[0m[47m[30m         [0m[40m[37m',
            '[0m[41m[30m [0m[47m[30m        [0m[40m[37m',
            '[0m[41m[30m  [0m[47m[30m       [0m[40m[37m',
            '[0m[41m[30m   [0m[47m[30m      [0m[40m[37m',
            '[0m[43m[30m    [0m[47m[30m     [0m[40m[37m',
            '[0m[43m[30m     [0m[47m[30m    [0m[40m[37m',
            '[0m[43m[30m      [0m[47m[30m   [0m[40m[37m',
            '[0m[42m[30m       [0m[47m[30m  [0m[40m[37m',
            '[0m[42m[30m        [0m[47m[30m [0m[40m[37m',
            '[0m[42m[30m         [0m[47m[30m[0m[40m[37m',
        ]
        hp = ob.getHp()
        maxhp = ob.getMaxHp()
       
        p=int(hp*9//maxhp)
        if p <0:
            p = 0
        elif p>9:
            p=9
        msg = ('%3d ' % int(hp*100//maxhp)) + strAnsi[p]
        return msg
                
    def viewParty(self, ob):
        if ob.Party == None:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('◁ [1m%s[0m[40m[37m의 동행 ▷' % ob['이름'])
            ob.sendLine('────────────────────────────')
        else:
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            ob.sendLine('◁ [1m%s[0m[40m[37m의 무리 ▷' % ob.Party['이름'])
            ob.sendLine('────────────────────────────')
        
        #buff = '[%s]' % szStr
        msg=''
        if ob.Party != None:
            nick=ob.Party['무림별호']
            if(nick==''):
                nick="무명객"
            if ob.Party['성격'] != '기인':
                msg = '▶ [1m[33m[40m%-12s[0m[37m[40m %-10s  ' % (nick, ob.Party['이름'])
            else:
                msg = '▶ %-12s %-10s  ' % (nick, ob.Party['이름'])
            mp = ob.Party.getMp()
            maxmp = ob.Party.getMaxMp()
            msg += '%-19s  ' % self.getHPbar(ob.Party)
            ob.sendLine(msg + '%5d/%-5d' % (mp, maxmp))
            for member in ob.Party.PartyMember:
                msg=''
                nick=member['무림별호']
                if(nick==''):
                    nick="무명객"
                if member['성격'] != '기인':
                    msg = '　 [1m[33m[40m%-12s[0m[37m[40m %-10s  ' % (nick, member['이름'])
                else:
                    msg = '　 %-12s %-10s  ' % (nick, member['이름'])
                mp = member.getMp()
                maxmp = member.getMaxMp()
                msg += '%-19s  ' % self.getHPbar(member)
                ob.sendLine(msg + '%5d/%-5d' % (mp, maxmp))
            ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
        ob.sendLine('동행중')
        for member in ob.follower:
            if ob.Party != None:
                if member in ob.Party.PartyMember:
                    continue
            msg=''
            nick=member['무림별호']
            if(nick==''):
                nick="무명객"
            if member['성격'] != '기인':
                msg = '　 [1m[33m[40m%-12s[0m[37m[40m %-10s  ' % (nick, member['이름'])
            else:
                msg = '　 %-12s %-10s  ' % (nick, member['이름'])
            mp = member.getMp()
            maxmp = member.getMaxMp()
            msg += '%-19s  ' % self.getHPbar(member)
            ob.sendLine(msg + '%5d/%-5d' % (mp, maxmp))
        ob.sendLine('━━━━━━━━━━━━━━━━━━━━━━━━━━━━')
            

    def cmd(self, ob, line):
        if line == '':
            if ob.Party == None and ob.follower == None :
                ob.sendLine('☞ 당신이 속한 무리가 없어요. ^^')
            else:
                self.viewParty(ob)
            return
        else:
            self.addParty(ob, line)
            
